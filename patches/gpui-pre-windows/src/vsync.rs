use std::{
    sync::{Condvar, LazyLock, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use gpui::FrameRequestSource;
use gpui_util::ResultExt;
use windows::Win32::{
    Foundation::HWND,
    Graphics::Dwm::{DWM_TIMING_INFO, DwmFlush, DwmGetCompositionTimingInfo},
    System::Performance::QueryPerformanceFrequency,
};

static QPC_TICKS_PER_SECOND: LazyLock<u64> = LazyLock::new(|| {
    let mut frequency = 0;
    // On systems that run Windows XP or later, the function will always succeed and
    // will thus never return zero.
    unsafe { QueryPerformanceFrequency(&mut frequency).unwrap() };
    frequency as u64
});

/// Where the vsync thread sleeps while every window is minimised or hidden,
/// instead of waking on every vsync for windows that draw nothing. One per
/// process, like the vsync thread.
pub(crate) static VSYNC_GATE: VSyncGate = VSyncGate {
    opened: Mutex::new(false),
    condvar: Condvar::new(),
};

pub(crate) struct VSyncGate {
    opened: Mutex<bool>,
    condvar: Condvar,
}

impl VSyncGate {
    /// Wakes the vsync thread if it sleeps, or keeps its next wait from sleeping.
    pub(crate) fn open(&self) {
        *self.opened.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.condvar.notify_all();
    }

    /// Sleeps while `idle` holds and nothing opened the gate since the last
    /// wait. `idle` must not wait on the main thread.
    pub(crate) fn wait_while(&self, idle: impl Fn() -> bool) {
        let mut opened = self.opened.lock().unwrap_or_else(|e| e.into_inner());
        while !*opened && idle() {
            opened = self.condvar.wait(opened).unwrap_or_else(|e| e.into_inner());
        }
        *opened = false;
    }
}

const VSYNC_INTERVAL_THRESHOLD: Duration = Duration::from_millis(1);
const DEFAULT_VSYNC_INTERVAL: Duration = Duration::from_micros(16_666); // ~60Hz

pub(crate) struct VSyncProvider {
    interval: Duration,
    f: Box<dyn Fn() -> bool>,
}

impl VSyncProvider {
    pub(crate) fn new() -> Self {
        let interval = get_dwm_interval()
            .context("Failed to get DWM interval")
            .log_err()
            .unwrap_or(DEFAULT_VSYNC_INTERVAL);
        let f = Box::new(|| unsafe { DwmFlush().is_ok() });
        Self { interval, f }
    }

    pub(crate) fn wait_for_vsync(&self) -> FrameRequestSource {
        let vsync_start = Instant::now();
        let wait_succeeded = (self.f)();
        let elapsed = vsync_start.elapsed();
        // DwmFlush and DCompositionWaitForCompositorClock returns very early
        // instead of waiting until vblank when the monitor goes to sleep or is
        // unplugged (nothing to present due to desktop occlusion). We use 1ms as
        // a threshold for the duration of the wait functions and fallback to
        // Sleep() if it returns before that. This could happen during normal
        // operation for the first call after the vsync thread becomes non-idle,
        // but it shouldn't happen often.
        if !wait_succeeded || elapsed < VSYNC_INTERVAL_THRESHOLD {
            log::trace!("VSyncProvider::wait_for_vsync() took less time than expected");
            std::thread::sleep(self.interval);
            FrameRequestSource::LocalSchedule
        } else {
            FrameRequestSource::NativeCallback
        }
    }
}

fn get_dwm_interval() -> Result<Duration> {
    let mut timing_info = DWM_TIMING_INFO {
        cbSize: std::mem::size_of::<DWM_TIMING_INFO>() as u32,
        ..Default::default()
    };
    unsafe { DwmGetCompositionTimingInfo(HWND::default(), &mut timing_info) }?;
    let interval = retrieve_duration(timing_info.qpcRefreshPeriod, *QPC_TICKS_PER_SECOND);
    // Check for interval values that are impossibly low. A 29 microsecond
    // interval was seen (from a qpcRefreshPeriod of 60).
    if interval < VSYNC_INTERVAL_THRESHOLD {
        Ok(retrieve_duration(
            timing_info.rateRefresh.uiDenominator as u64,
            timing_info.rateRefresh.uiNumerator as u64,
        ))
    } else {
        Ok(interval)
    }
}

#[inline]
fn retrieve_duration(counts: u64, ticks_per_second: u64) -> Duration {
    let ticks_per_microsecond = ticks_per_second / 1_000_000;
    Duration::from_micros(counts / ticks_per_microsecond)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_compositor_wait_reports_native_callback() {
        let provider = VSyncProvider {
            interval: Duration::ZERO,
            f: Box::new(|| {
                std::thread::sleep(VSYNC_INTERVAL_THRESHOLD * 2);
                true
            }),
        };

        assert_eq!(
            provider.wait_for_vsync(),
            FrameRequestSource::NativeCallback
        );
    }

    #[test]
    fn failed_compositor_wait_reports_local_schedule_even_after_threshold() {
        let provider = VSyncProvider {
            interval: Duration::ZERO,
            f: Box::new(|| {
                std::thread::sleep(VSYNC_INTERVAL_THRESHOLD * 2);
                false
            }),
        };

        assert_eq!(provider.wait_for_vsync(), FrameRequestSource::LocalSchedule);
    }

    #[test]
    fn the_gate_sleeps_only_while_idle_and_not_opened() {
        let gate = VSyncGate {
            opened: Mutex::new(false),
            condvar: Condvar::new(),
        };
        gate.wait_while(|| false); // not idle: returns at once
        gate.open();
        gate.wait_while(|| true); // opened before the wait: returns at once
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(20));
                gate.open();
            });
            gate.wait_while(|| true); // idle: sleeps until opened
        });
    }
}
