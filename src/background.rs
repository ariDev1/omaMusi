//! One worker and one replaceable pending task per operation category.
use anyhow::{Result, anyhow};
use std::{
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Condvar, Mutex},
    thread,
};

type Operation<T> = Box<dyn FnOnce() -> Result<T> + Send + 'static>;
struct Slot<T> {
    latest: Option<u64>,
    pending: Option<(u64, Operation<T>)>,
    result: Option<TaskResult<T>>,
}
struct State<T> {
    serial: u64,
    closed: bool,
    slots: HashMap<String, Slot<T>>,
}
struct Shared<T> {
    state: Mutex<State<T>>,
    changed: Condvar,
}
pub struct TaskResult<T> {
    pub category: String,
    pub generation: u64,
    pub result: Result<T>,
}
pub struct BackgroundTasks<T: Send + 'static> {
    shared: Arc<Shared<T>>,
}

impl<T: Send + 'static> Default for BackgroundTasks<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Send + 'static> BackgroundTasks<T> {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    serial: 0,
                    closed: false,
                    slots: HashMap::new(),
                }),
                changed: Condvar::new(),
            }),
        }
    }
    pub fn submit(
        &self,
        category: &str,
        operation: impl FnOnce() -> Result<T> + Send + 'static,
    ) -> Option<u64> {
        let mut state = self.shared.state.lock().unwrap();
        if state.closed {
            return None;
        }
        state.serial += 1;
        let generation = state.serial;
        let fresh = !state.slots.contains_key(category);
        let slot = state.slots.entry(category.to_owned()).or_insert(Slot {
            latest: None,
            pending: None,
            result: None,
        });
        slot.latest = Some(generation);
        slot.pending = Some((generation, Box::new(operation)));
        slot.result = None;
        if fresh {
            let shared = self.shared.clone();
            let category = category.to_owned();
            thread::Builder::new()
                .name(format!("omaMusi {category}"))
                .spawn(move || worker(shared, category))
                .expect("could not start background worker");
        }
        self.shared.changed.notify_all();
        Some(generation)
    }
    pub fn invalidate(&self, category: &str) {
        if let Some(slot) = self.shared.state.lock().unwrap().slots.get_mut(category) {
            slot.latest = None;
            slot.pending = None;
            slot.result = None;
        }
    }
    pub fn poll(&self) -> Vec<TaskResult<T>> {
        self.shared
            .state
            .lock()
            .unwrap()
            .slots
            .values_mut()
            .filter_map(|slot| slot.result.take())
            .collect()
    }
    pub fn close(&self) {
        let mut state = self.shared.state.lock().unwrap();
        state.closed = true;
        for slot in state.slots.values_mut() {
            slot.pending = None;
            slot.result = None;
            slot.latest = None;
        }
        self.shared.changed.notify_all();
    }
}
impl<T: Send + 'static> Drop for BackgroundTasks<T> {
    fn drop(&mut self) {
        self.close();
    }
}

fn worker<T: Send + 'static>(shared: Arc<Shared<T>>, category: String) {
    loop {
        let (generation, operation) = {
            let mut state = shared.state.lock().unwrap();
            loop {
                if state.closed {
                    return;
                }
                if let Some(task) = state.slots.get_mut(&category).unwrap().pending.take() {
                    break task;
                }
                state = shared.changed.wait(state).unwrap();
            }
        };
        let result = catch_unwind(AssertUnwindSafe(operation))
            .unwrap_or_else(|_| Err(anyhow!("Background operation panicked")));
        let mut state = shared.state.lock().unwrap();
        if state.closed {
            return;
        }
        let slot = state.slots.get_mut(&category).unwrap();
        if slot.latest == Some(generation) {
            slot.result = Some(TaskResult {
                category: category.clone(),
                generation,
                result,
            });
        }
    }
}
