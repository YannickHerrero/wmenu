use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TICK: Duration = Duration::from_secs(240);
const NUDGE_PX: f32 = 20.0;

pub struct Amphetamine {
    enabled: Arc<AtomicBool>,
}

impl Amphetamine {
    pub fn new(initial: bool) -> Self {
        let enabled = Arc::new(AtomicBool::new(initial));
        let worker_enabled = enabled.clone();
        thread::spawn(move || worker(worker_enabled));
        Self { enabled }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set(&self, on: bool) {
        self.enabled.store(on, Ordering::Relaxed);
    }
}

fn worker(enabled: Arc<AtomicBool>) {
    loop {
        thread::sleep(TICK);
        if !enabled.load(Ordering::Relaxed) {
            continue;
        }
        nudge_cursor();
    }
}

#[cfg(windows)]
fn nudge_cursor() {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

    let mut pt = POINT { x: 0, y: 0 };
    unsafe {
        if GetCursorPos(&mut pt).is_err() {
            return;
        }
    }

    // Pseudo-random direction from the system clock — true randomness
    // is overkill for once-every-four-minutes.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let angle = (nanos % 360) as f32 * std::f32::consts::PI / 180.0;
    let dx = (angle.cos() * NUDGE_PX) as i32;
    let dy = (angle.sin() * NUDGE_PX) as i32;

    let _ = unsafe { SetCursorPos(pt.x + dx, pt.y + dy) };
}

#[cfg(not(windows))]
fn nudge_cursor() {}
