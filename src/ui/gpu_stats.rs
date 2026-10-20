//! Linux hardware counters for the text-only GPU Lab readout.
use std::{
    fs,
    path::{Path, PathBuf},
};

fn number(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn field(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let value = line.strip_prefix(key)?.trim_start();
        value.split_whitespace().next()?.parse().ok()
    })
}

fn cpu_times(text: &str) -> Option<(u64, u64)> {
    let mut times = text
        .lines()
        .next()?
        .strip_prefix("cpu ")?
        .split_whitespace();
    let values: Vec<u64> = (0..8)
        .map(|_| times.next()?.parse().ok())
        .collect::<Option<_>>()?;
    // iowait is idle; guest time is already included in user/nice.
    let total = values.iter().sum();
    Some((total, values[3] + values[4]))
}

fn cpu_temperature() -> Option<f64> {
    for entry in fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
        let path = entry.path();
        let Ok(name) = fs::read_to_string(path.join("name")) else {
            continue;
        };
        if matches!(name.trim(), "coretemp" | "k10temp" | "zenpower") {
            return number(&path.join("temp1_input")).map(|v| v as f64 / 1000.0);
        }
    }
    None
}

fn gpu_device() -> Option<PathBuf> {
    let devices: Vec<_> = fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let suffix = name.strip_prefix("card")?;
            (suffix.bytes().all(|b| b.is_ascii_digit())
                && !suffix.is_empty()
                && entry.path().join("device").exists())
            .then(|| entry.path().join("device"))
        })
        .collect();
    // No reliable mapping between a GL context and one of several DRM cards.
    (devices.len() == 1)
        .then(|| devices.into_iter().next())
        .flatten()
}

fn hwmon(device: &Path) -> Option<PathBuf> {
    fs::read_dir(device.join("hwmon"))
        .ok()?
        .flatten()
        .next()
        .map(|e| e.path())
}

fn clock(hwmon: &Path, label: &str) -> Option<u64> {
    for index in 1..=4 {
        let base = format!("freq{index}");
        if fs::read_to_string(hwmon.join(format!("{base}_label")))
            .ok()
            .is_some_and(|s| s.trim() == label)
        {
            return number(&hwmon.join(format!("{base}_input"))).map(|hz| hz / 1_000_000);
        }
    }
    None
}

#[derive(Default)]
struct Samples {
    sum: u64,
    count: u64,
    peak: u64,
}
impl Samples {
    fn add(&mut self, value: Option<u64>) {
        if let Some(value) = value {
            self.sum += value;
            self.count += 1;
            self.peak = self.peak.max(value);
        }
    }
    fn display(&mut self) -> String {
        let result = self.sum.checked_div(self.count).map_or_else(
            || "n/a".into(),
            |average| format!("{average}% avg / {}% peak", self.peak),
        );
        *self = Self::default();
        result
    }
}

pub struct Telemetry {
    gpu: Option<PathBuf>,
    busy: Samples,
    memory_busy: Samples,
    cpu_previous: Option<(u64, u64)>,
}
impl Telemetry {
    pub fn new() -> Self {
        Self {
            gpu: gpu_device(),
            busy: Samples::default(),
            memory_busy: Samples::default(),
            cpu_previous: None,
        }
    }

    pub fn reset(&mut self) {
        self.busy = Samples::default();
        self.memory_busy = Samples::default();
        self.cpu_previous = fs::read_to_string("/proc/stat")
            .ok()
            .and_then(|s| cpu_times(&s));
    }

    pub fn sample(&mut self) {
        if let Some(gpu) = &self.gpu {
            self.busy.add(number(&gpu.join("gpu_busy_percent")));
            self.memory_busy.add(number(&gpu.join("mem_busy_percent")));
        }
    }

    pub fn report(&mut self) -> String {
        let Some(device) = &self.gpu else {
            return format!(
                "GPU  sensors unavailable (multiple GPUs or unsupported driver)\n{}",
                self.cpu_report()
            );
        };
        let busy = self.busy.display();
        let memory_busy = self.memory_busy.display();
        let used = number(&device.join("mem_info_vram_used"));
        let total = number(&device.join("mem_info_vram_total"));
        let memory = match (used, total) {
            (Some(used), Some(total)) if total > 0 => format!(
                "{:.0}/{:.0} MiB ({:.0}%)",
                used as f64 / 1_048_576.0,
                total as f64 / 1_048_576.0,
                used as f64 * 100.0 / total as f64
            ),
            _ => "n/a".into(),
        };
        let monitor = hwmon(device);
        let sensor = |name: &str| monitor.as_ref().and_then(|p| number(&p.join(name)));
        let frequency = |label: &str| monitor.as_ref().and_then(|p| clock(p, label));
        let show = |value: Option<u64>, unit: &str| {
            value.map_or_else(|| "n/a".into(), |n| format!("{n} {unit}"))
        };
        let temp = sensor("temp1_input").map(|v| v as f64 / 1000.0);
        let power = sensor("power1_input").map(|v| v as f64 / 1_000_000.0);
        let fan = sensor("fan1_input");
        format!(
            "GPU  core busy {busy}\nGPU  VRAM {memory}  ·  memory busy {memory_busy}\nGPU  core {}  ·  memory {}\nGPU  {}  ·  {}  ·  fan {}\n{}",
            show(frequency("sclk"), "MHz"),
            show(frequency("mclk"), "MHz"),
            temp.map_or_else(|| "n/a".into(), |v| format!("{v:.0}°C")),
            power.map_or_else(|| "n/a".into(), |v| format!("{v:.1} W")),
            show(fan, "RPM"),
            self.cpu_report(),
        )
    }

    fn cpu_report(&mut self) -> String {
        let current = fs::read_to_string("/proc/stat")
            .ok()
            .and_then(|s| cpu_times(&s));
        let load =
            self.cpu_previous
                .zip(current)
                .and_then(|((old_total, old_idle), (total, idle))| {
                    let span = total.checked_sub(old_total)?;
                    (span > 0).then(|| {
                        (span.saturating_sub(idle.saturating_sub(old_idle))) as f64 * 100.0
                            / span as f64
                    })
                });
        self.cpu_previous = current;
        let info = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let model = info
            .lines()
            .find_map(|line| {
                line.strip_prefix("model name")
                    .and_then(|s| s.split_once(':'))
                    .map(|(_, s)| s.trim())
            })
            .unwrap_or("CPU");
        let ghz = info.lines().find_map(|line| {
            line.strip_prefix("cpu MHz")
                .and_then(|s| s.split_once(':'))
                .and_then(|(_, s)| s.trim().parse::<f64>().ok())
        });
        let mem = fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let (total, available) = (field(&mem, "MemTotal:"), field(&mem, "MemAvailable:"));
        let ram = match (total, available) {
            (Some(total), Some(available)) => format!(
                "{:.1}/{:.1} GiB",
                (total - available.min(total)) as f64 / 1_048_576.0,
                total as f64 / 1_048_576.0
            ),
            _ => "n/a".into(),
        };
        let process = fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| field(&s, "VmRSS:"))
            .map_or_else(
                || "n/a".into(),
                |kb| format!("{:.0} MiB", kb as f64 / 1024.0),
            );
        let threads = std::thread::available_parallelism()
            .map(|v| v.get())
            .unwrap_or(0);
        format!(
            "CPU  {model}\nCPU  {threads} threads available  ·  {}  ·  {}  ·  {}\nRAM  {ram}  ·  omaMusi {process}",
            load.map_or_else(|| "n/a".into(), |v| format!("{v:.0}% load")),
            ghz.map_or_else(|| "n/a".into(), |v| format!("core0 {v:.0} MHz")),
            cpu_temperature().map_or_else(|| "n/a".into(), |v| format!("{v:.0}°C")),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_usage_uses_jiffy_deltas_and_iowait_as_idle() {
        let before = cpu_times("cpu  10 2 3 15 5 0 0 0 0 0\n").unwrap();
        let after = cpu_times("cpu  15 2 8 20 15 0 0 0 0 0\n").unwrap();
        assert_eq!((after.0 - before.0, after.1 - before.1), (25, 15));
    }

    #[test]
    fn sensor_samples_report_average_and_peak_without_inventing_missing_values() {
        let mut samples = Samples::default();
        assert_eq!(samples.display(), "n/a");
        samples.add(Some(0));
        samples.add(None);
        samples.add(Some(20));
        assert_eq!(samples.display(), "10% avg / 20% peak");
        assert_eq!(samples.display(), "n/a");
    }
}
