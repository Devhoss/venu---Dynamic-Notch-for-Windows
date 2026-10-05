//! Lightweight system telemetry for the Stats page and notch slide.
//!
//! Sampling is throttled to once per second. CPU/RAM/power use direct Win32
//! APIs; GPU uses Windows' PDH "GPU Engine" counters and reports the busiest
//! engine, which matches the useful "how busy is the GPU right now?" view
//! without pretending to know board power or temperature.

use parking_lot::Mutex;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use windows::core::PCSTR;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterA, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayA,
    PdhOpenQueryA, PDH_FMT_COUNTERVALUE_ITEM_A, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
    PDH_MORE_DATA,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

const SAMPLE_INTERVAL: Duration = Duration::from_millis(900);
const GPU_COUNTER: &[u8] = b"\\GPU Engine(*)\\Utilization Percentage\0";

#[derive(Debug, Clone, Default)]
pub struct StatsSnapshot {
    pub revision: u64,
    pub cpu_pct: Option<f32>,
    pub ram_pct: Option<f32>,
    pub ram_used: u64,
    pub ram_total: u64,
    pub gpu_pct: Option<f32>,
    pub ac_online: Option<bool>,
    pub battery_pct: Option<u8>,
    pub charging: bool,
}

struct GpuQuery {
    query: usize,
    counter: usize,
}

impl GpuQuery {
    fn new() -> Option<Self> {
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryA(PCSTR::null(), 0, &mut query) != 0 {
                return None;
            }

            let mut counter = PDH_HCOUNTER::default();
            let status = PdhAddEnglishCounterA(
                query,
                PCSTR(GPU_COUNTER.as_ptr()),
                0,
                &mut counter,
            );
            if status != 0 {
                let _ = PdhCloseQuery(query);
                return None;
            }

            // Percentage counters need two samples. Prime the query now; the
            // next one-second Stats refresh can return a real value.
            let _ = PdhCollectQueryData(query);
            Some(Self {
                query: query.0 as usize,
                counter: counter.0 as usize,
            })
        }
    }

    fn sample(&mut self) -> Option<f32> {
        unsafe {
            let query = PDH_HQUERY(self.query as *mut _);
            let counter = PDH_HCOUNTER(self.counter as *mut _);
            if PdhCollectQueryData(query) != 0 {
                return None;
            }

            let mut bytes = 0u32;
            let mut count = 0u32;
            let first = PdhGetFormattedCounterArrayA(
                counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                None,
            );
            if first != PDH_MORE_DATA || bytes == 0 || count == 0 {
                return None;
            }

            // PDH writes the item array plus its strings into one caller-owned
            // byte buffer. usize gives the allocation pointer enough alignment
            // for PDH_FMT_COUNTERVALUE_ITEM_A.
            let words = (bytes as usize + std::mem::size_of::<usize>() - 1)
                / std::mem::size_of::<usize>();
            let mut buffer = vec![0usize; words];
            let items = buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_A;
            let status = PdhGetFormattedCounterArrayA(
                counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                Some(items),
            );
            if status != 0 {
                return None;
            }

            let slice = std::slice::from_raw_parts(items, count as usize);
            let mut busiest = 0.0f64;
            let mut found = false;
            for item in slice {
                // 0 = valid data, 1 = new data. Ignore stale/error entries.
                if item.FmtValue.CStatus > 1 {
                    continue;
                }
                let value = item.FmtValue.Anonymous.doubleValue;
                if value.is_finite() {
                    busiest = busiest.max(value);
                    found = true;
                }
            }

            found.then(|| busiest.clamp(0.0, 100.0) as f32)
        }
    }
}

impl Drop for GpuQuery {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(PDH_HQUERY(self.query as *mut _));
        }
    }
}

struct Sampler {
    last_sample: Option<Instant>,
    previous_cpu: Option<(u64, u64, u64)>,
    gpu: Option<GpuQuery>,
    snapshot: StatsSnapshot,
}

impl Default for Sampler {
    fn default() -> Self {
        Self {
            last_sample: None,
            previous_cpu: None,
            gpu: None,
            snapshot: StatsSnapshot::default(),
        }
    }
}

impl Sampler {
    fn refresh_if_due(&mut self) {
        if self
            .last_sample
            .is_some_and(|at| at.elapsed() < SAMPLE_INTERVAL)
        {
            return;
        }
        self.last_sample = Some(Instant::now());
        self.snapshot.revision = self.snapshot.revision.wrapping_add(1);

        self.sample_cpu();
        self.sample_memory();
        self.sample_power();

        if self.gpu.is_none() {
            self.gpu = GpuQuery::new();
        }
        self.snapshot.gpu_pct = self.gpu.as_mut().and_then(GpuQuery::sample);
    }

    fn sample_cpu(&mut self) {
        let mut idle = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        if unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.is_err() {
            self.snapshot.cpu_pct = None;
            return;
        }

        let current = (
            filetime_ticks(idle),
            filetime_ticks(kernel),
            filetime_ticks(user),
        );
        self.snapshot.cpu_pct = self.previous_cpu.and_then(|previous| {
            let idle_delta = current.0.saturating_sub(previous.0);
            let kernel_delta = current.1.saturating_sub(previous.1);
            let user_delta = current.2.saturating_sub(previous.2);
            let total = kernel_delta.saturating_add(user_delta);
            if total == 0 {
                None
            } else {
                Some(
                    (100.0 * (total.saturating_sub(idle_delta)) as f64 / total as f64)
                        .clamp(0.0, 100.0) as f32,
                )
            }
        });
        self.previous_cpu = Some(current);
    }

    fn sample_memory(&mut self) {
        let mut memory = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if unsafe { GlobalMemoryStatusEx(&mut memory) }.is_ok() {
            self.snapshot.ram_total = memory.ullTotalPhys;
            self.snapshot.ram_used = memory.ullTotalPhys.saturating_sub(memory.ullAvailPhys);
            self.snapshot.ram_pct = Some(memory.dwMemoryLoad.min(100) as f32);
        } else {
            self.snapshot.ram_pct = None;
            self.snapshot.ram_used = 0;
            self.snapshot.ram_total = 0;
        }
    }

    fn sample_power(&mut self) {
        let mut power = SYSTEM_POWER_STATUS::default();
        if unsafe { GetSystemPowerStatus(&mut power) }.is_err() {
            self.snapshot.ac_online = None;
            self.snapshot.battery_pct = None;
            self.snapshot.charging = false;
            return;
        }

        self.snapshot.ac_online = match power.ACLineStatus {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };
        self.snapshot.battery_pct =
            (power.BatteryLifePercent != u8::MAX).then_some(power.BatteryLifePercent);
        // SYSTEM_POWER_STATUS BatteryFlag bit 3 means charging.
        self.snapshot.charging = power.BatteryFlag & 0x08 != 0;
    }
}

fn filetime_ticks(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

static SAMPLER: OnceLock<Mutex<Sampler>> = OnceLock::new();

pub fn snapshot() -> StatsSnapshot {
    let mut sampler = SAMPLER.get_or_init(|| Mutex::new(Sampler::default())).lock();
    sampler.refresh_if_due();
    sampler.snapshot.clone()
}

pub fn format_bytes(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= GIB {
        format!("{:.1} GB", bytes as f64 / GIB)
    } else {
        format!("{:.0} MB", bytes as f64 / MIB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_format_is_compact() {
        assert_eq!(format_bytes(8 * 1024 * 1024 * 1024), "8.0 GB");
        assert_eq!(format_bytes(512 * 1024 * 1024), "512 MB");
    }
}
