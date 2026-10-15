//! Bounded FFmpeg PCM producer. No file or process work runs on the audio callback.
use anyhow::{Result, bail};
use rtrb::{Consumer, RingBuffer};
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub const PCM_CAPACITY: usize = 65_536;
pub struct DecodeState {
    pub done: AtomicBool,
    cancelled: AtomicBool,
    error: Mutex<Option<String>>,
}
pub struct Decoder {
    process: Arc<Mutex<Child>>,
    worker: Option<JoinHandle<()>>,
    consumer: Option<Consumer<f32>>,
    pub(crate) state: Arc<DecodeState>,
    pid: u32,
}

impl DecodeState {
    pub(super) fn new() -> Self {
        Self {
            done: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            error: Mutex::new(None),
        }
    }
}

impl Decoder {
    pub fn spawn(path: &Path, offset: f64, rate: u32, channels: u16) -> Result<Self> {
        if !offset.is_finite() || offset < 0.0 || rate == 0 || channels == 0 || channels > 32 {
            bail!("Invalid decoder parameters");
        }
        let mut child = Command::new("ffmpeg")
            .args(["-nostdin", "-v", "error", "-ss", &offset.to_string(), "-i"])
            .arg(path)
            .args([
                "-vn",
                "-f",
                "f32le",
                "-acodec",
                "pcm_f32le",
                "-ar",
                &rate.to_string(),
                "-ac",
                &channels.to_string(),
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let pid = child.id();
        let process = Arc::new(Mutex::new(child));
        let (mut producer, consumer) = RingBuffer::new(PCM_CAPACITY);
        let state = Arc::new(DecodeState::new());
        let worker_state = state.clone();
        let worker_process = process.clone();
        let worker = thread::spawn(move || {
            let errors = thread::spawn(move || {
                let mut buffer = [0u8; 1024];
                let mut tail = Vec::new();
                while let Ok(count) = stderr.read(&mut buffer) {
                    if count == 0 {
                        break;
                    }
                    tail.extend_from_slice(&buffer[..count]);
                    if tail.len() > 8192 {
                        tail.drain(..tail.len() - 8192);
                    }
                }
                String::from_utf8_lossy(&tail).trim().to_owned()
            });
            let frame_bytes = channels as usize * 4;
            let mut buffer = vec![0u8; 8192 + 128];
            let mut retained = 0;
            'decode: loop {
                if worker_state.cancelled.load(Ordering::Acquire) || producer.is_abandoned() {
                    break;
                }
                let count = match stdout.read(&mut buffer[retained..8192 + retained]) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        *worker_state.error.lock().unwrap() = Some(e.to_string());
                        break;
                    }
                };
                let bytes = retained + count;
                let complete = bytes / frame_bytes * frame_bytes;
                for frame in buffer[..complete].chunks_exact(frame_bytes) {
                    while producer.slots() < channels as usize {
                        if worker_state.cancelled.load(Ordering::Acquire) || producer.is_abandoned()
                        {
                            break 'decode;
                        }
                        thread::sleep(Duration::from_millis(1));
                    }
                    for sample in frame.as_chunks::<4>().0 {
                        let value = f32::from_le_bytes(*sample);
                        producer
                            .push(if value.is_finite() { value } else { 0.0 })
                            .unwrap();
                    }
                }
                retained = bytes - complete;
                buffer.copy_within(complete..bytes, 0);
            }
            if worker_state.cancelled.load(Ordering::Acquire) || producer.is_abandoned() {
                let _ = worker_process.lock().unwrap().kill();
            }
            let status = loop {
                match worker_process.lock().unwrap().try_wait() {
                    Ok(Some(status)) => break Some(status),
                    Ok(None) => {}
                    Err(_) => break None,
                }
                if worker_state.cancelled.load(Ordering::Acquire) {
                    let _ = worker_process.lock().unwrap().kill();
                }
                thread::sleep(Duration::from_millis(2));
            };
            let error = errors
                .join()
                .unwrap_or_else(|_| "FFmpeg error reader failed".into());
            if !worker_state.cancelled.load(Ordering::Acquire)
                && status.is_none_or(|s| !s.success())
            {
                *worker_state.error.lock().unwrap() = Some(if error.is_empty() {
                    "FFmpeg could not decode this file".into()
                } else {
                    error
                });
            }
            worker_state.done.store(true, Ordering::Release);
        });
        Ok(Self {
            process,
            worker: Some(worker),
            consumer: Some(consumer),
            state,
            pid,
        })
    }
    pub fn take_consumer(&mut self) -> Option<Consumer<f32>> {
        self.consumer.take()
    }
    pub fn done(&self) -> bool {
        self.state.done.load(Ordering::Acquire)
    }
    pub fn error(&self) -> Option<String> {
        self.state.error.lock().unwrap().clone()
    }
    pub fn process_id(&self) -> u32 {
        self.pid
    }
    pub fn close(&mut self) {
        self.state.cancelled.store(true, Ordering::Release);
        let _ = self.process.lock().unwrap().kill();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // The worker uses try_wait(), which reaps the child; close is idempotent.
        self.consumer.take();
    }
}
impl Drop for Decoder {
    fn drop(&mut self) {
        self.close();
    }
}
