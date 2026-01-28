use rand::Rng;
use rdev::{listen, Event, EventType, Key};
use std::{
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

fn xdotool(args: &[&str]) {
    let _ = Command::new("xdotool").args(args).status();
}

fn key_down(key: &str) {
    xdotool(&["keydown", key]);
}
fn key_up(key: &str) {
    xdotool(&["keyup", key]);
}

fn mouse_down(button: u8) {
    xdotool(&["mousedown", &button.to_string()]);
}

fn mouse_up(button: u8) {
    xdotool(&["mouseup", &button.to_string()]);
}

fn click(button: u8) {
    xdotool(&["click", &button.to_string()]);
}

fn release_safety() {
    key_up("s");
    key_up("f");
    key_up("q");
    key_up("c");
    key_up("Shift_L");
    mouse_up(1);
    mouse_up(2);
    mouse_up(3);
}

fn fmt_elapsed(start: Instant) -> String {
    let secs = start.elapsed().as_secs();
    let mm = secs / 60;
    let ss = secs % 60;
    format!("{:02}:{:02}", mm, ss)
}

fn log_elapsed(start: Instant, label: &str) {
    println!("{} | {}", label, fmt_elapsed(start));
}

#[derive(Clone)]
struct Ctrl {
    running: Arc<AtomicBool>,
    run_id: Arc<AtomicU64>,
    my_id: u64,
}

impl Ctrl {
    fn stop_requested(&self) -> bool {
        !self.running.load(Ordering::Relaxed) || self.run_id.load(Ordering::Relaxed) != self.my_id
    }

    fn sleep_interruptible(&self, secs: f64) -> bool {
        if secs <= 0.0 {
            return !self.stop_requested();
        }
        let total = Duration::from_secs_f64(secs);
        let tick = Duration::from_millis(5);
        let start = Instant::now();

        while start.elapsed() < total {
            if self.stop_requested() {
                release_safety();
                return false;
            }
            let remaining = total.saturating_sub(start.elapsed());
            thread::sleep(std::cmp::min(tick, remaining));
        }
        true
    }

    fn sleep_range(&self, min: f64, max: f64) -> bool {
        let mut rng = rand::thread_rng();
        let secs = rng.gen_range(min..max);
        self.sleep_interruptible(secs)
    }

    fn tap_key(&self, key: &str) -> bool {
        if self.stop_requested() {
            release_safety();
            return false;
        }
        key_down(key);
        if !self.sleep_interruptible(0.01) {
            return false;
        }
        key_up(key);
        true
    }
}

#[derive(Debug)]
struct BuffCooldown {
    last_q: Option<Instant>,
    last_2: Option<Instant>,
    last_3: Option<Instant>,
    last_4: Option<Instant>,
}

impl BuffCooldown {
    fn new() -> Self {
        Self {
            last_q: None,
            last_2: None,
            last_3: None,
            last_4: None,
        }
    }

    fn ready(last: Option<Instant>, cd: Duration) -> bool {
        match last {
            None => true,
            Some(t) => t.elapsed() >= cd,
        }
    }

    fn press_if_ready(
        ctrl: &Ctrl,
        key: &str,
        last: &mut Option<Instant>,
        cd: Duration,
        after_sleep: f64,
    ) -> bool {
        if ctrl.stop_requested() {
            release_safety();
            return false;
        }
        if Self::ready(*last, cd) {
            if !ctrl.tap_key(key) {
                return false;
            }
            *last = Some(Instant::now());
            if !ctrl.sleep_interruptible(after_sleep) {
                return false;
            }
        }
        true
    }

    fn buff_once(&mut self, ctrl: &Ctrl) -> bool {
        if !Self::press_if_ready(ctrl, "q", &mut self.last_q, Duration::from_secs(60), 0.75) {
            return false;
        }
        if !Self::press_if_ready(ctrl, "2", &mut self.last_2, Duration::from_secs(60), 0.95) {
            return false;
        }
        if !Self::press_if_ready(ctrl, "3", &mut self.last_3, Duration::from_secs(60), 0.75) {
            return false;
        }
        if !Self::press_if_ready(ctrl, "4", &mut self.last_4, Duration::from_secs(180), 0.95) {
            return false;
        }

        if ctrl.stop_requested() {
            release_safety();
            return false;
        }

        if !ctrl.tap_key("z") {
            return false;
        }

        release_safety();
        true
    }
}

fn passive_skill(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("s");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    mouse_down(3);
    if !ctrl.sleep_range(0.45, 0.5) {
        return false;
    }
    mouse_up(3);
    key_up("s");
    true
}

fn combo_1_once(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("s");
    if !ctrl.sleep_range(0.05, 0.1) {
        return false;
    }
    click(1);
    key_up("s");

    if !ctrl.sleep_range(0.05, 0.06) {
        return false;
    }
    click(3);

    if !ctrl.sleep_range(0.5, 0.55) {
        return false;
    }
    click(3);

    if !ctrl.sleep_range(0.1, 0.2) {
        return false;
    }
    if !passive_skill(ctrl) {
        return false;
    }

    release_safety();
    true
}

fn combo_2_once(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("s");
    if !ctrl.sleep_range(0.05, 0.1) {
        return false;
    }
    key_down("f");
    key_up("f");
    key_up("s");

    if !ctrl.sleep_range(0.6, 0.65) {
        return false;
    }
    click(3);

    if !ctrl.sleep_range(1.1, 1.15) {
        return false;
    }
    click(3);

    if !ctrl.sleep_range(0.75, 0.8) {
        return false;
    }
    click(3);

    if !ctrl.sleep_range(0.75, 0.8) {
        return false;
    }
    click(3);

    if !ctrl.sleep_range(0.85, 0.9) {
        return false;
    }
    if !passive_skill(ctrl) {
        return false;
    }

    release_safety();
    true
}

fn strong_skill_1(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("Shift_L");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    mouse_down(1);
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    mouse_down(3);
    if !ctrl.sleep_range(2.2, 2.3) {
        return false;
    }

    key_up("Shift_L");
    mouse_up(1);
    mouse_up(3);

    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("f");
    if !ctrl.sleep_range(2.0, 2.05) {
        return false;
    }
    key_up("f");

    if !passive_skill(ctrl) {
        return false;
    }
    release_safety();
    true
}

fn strong_skill_2(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("Shift_L");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    key_down("f");
    if !ctrl.sleep_range(1.1, 1.15) {
        return false;
    }
    key_up("Shift_L");
    key_up("f");

    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("Shift_L");
    mouse_down(3);
    if !ctrl.sleep_range(1.65, 1.7) {
        return false;
    }
    key_up("Shift_L");
    mouse_up(3);

    if !passive_skill(ctrl) {
        return false;
    }

    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("s");
    if !ctrl.sleep_range(0.045, 0.05) {
        return false;
    }
    mouse_down(1);
    mouse_down(3);
    if !ctrl.sleep_range(1.2, 1.25) {
        return false;
    }
    mouse_up(1);
    mouse_up(3);
    if !ctrl.sleep_range(0.045, 0.05) {
        return false;
    }
    key_up("s");

    if !ctrl.sleep_range(0.5, 0.6) {
        return false;
    }
    mouse_up(3);

    release_safety();
    true
}

fn combo_3_once(ctrl: &Ctrl, use_strong_1: bool) -> bool {
    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("s");
    if !ctrl.sleep_range(0.05, 0.1) {
        return false;
    }
    key_down("c");
    key_up("s");
    key_up("c");

    if !ctrl.sleep_range(1.05, 1.1) {
        return false;
    }

    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    mouse_down(1);
    mouse_down(3);
    if !ctrl.sleep_range(0.01, 0.05) {
        return false;
    }
    mouse_up(1);
    mouse_up(3);

    if !ctrl.sleep_range(0.65, 0.7) {
        return false;
    }

    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    key_down("s");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    key_down("q");
    key_up("s");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    key_up("q");

    if ctrl.stop_requested() {
        release_safety();
        return false;
    }

    let ok: bool = if use_strong_1 {
        strong_skill_1(ctrl)
    } else {
        strong_skill_2(ctrl)
    };

    if !ok {
        return false;
    }

    release_safety();
    true
}

fn worker_loop(running: Arc<AtomicBool>, run_id: Arc<AtomicU64>, my_id: u64) {
    let ctrl = Ctrl {
        running,
        run_id,
        my_id,
    };

    let start = Instant::now();
    let mut round: u64 = 0;
    let mut buffs = BuffCooldown::new();

    if !buffs.buff_once(&ctrl) {
        release_safety();
        return;
    }
    round += 1;
    log_elapsed(start, &format!("Round #{round} (pre-buff)"));

    while !ctrl.stop_requested() {
        if !ctrl.sleep_range(0.25, 0.3) {
            break;
        }

        if !combo_1_once(&ctrl) {
            break;
        }
        if !ctrl.sleep_range(0.25, 0.3) {
            break;
        }

        if !combo_2_once(&ctrl) {
            break;
        }
        if !ctrl.sleep_range(0.25, 0.3) {
            break;
        }

        if !combo_3_once(&ctrl, true) {
            break;
        }
        if !ctrl.sleep_range(0.6, 0.65) {
            break;
        }

        if !combo_1_once(&ctrl) {
            break;
        }
        if !ctrl.sleep_range(0.25, 0.3) {
            break;
        }

        if !combo_2_once(&ctrl) {
            break;
        }
        if !ctrl.sleep_range(0.25, 0.3) {
            break;
        }

        if !combo_3_once(&ctrl, false) {
            break;
        }
        if !ctrl.sleep_range(0.25, 0.3) {
            break;
        }

        if !buffs.buff_once(&ctrl) {
            break;
        }

        release_safety();
        if !ctrl.sleep_range(0.25, 0.3) {
            break;
        }

        round += 1;
        log_elapsed(start, &format!("Round #{round} done"));
    }

    release_safety();
    log_elapsed(start, "Stopped");
}

fn main() {
    let running = Arc::new(AtomicBool::new(false));
    let run_id = Arc::new(AtomicU64::new(0));

    let running_cb = running.clone();
    let run_id_cb = run_id.clone();

    println!("F9 = Start loop, F10 = Stop");

    if let Err(err) = listen(move |event: Event| match event.event_type {
        EventType::KeyPress(key) => {
            if key == Key::F9 {
                let new_id = run_id_cb.fetch_add(1, Ordering::Relaxed) + 1;
                running_cb.store(true, Ordering::Relaxed);
                release_safety();

                let running2 = running_cb.clone();
                let run_id2 = run_id_cb.clone();
                thread::spawn(move || {
                    worker_loop(running2, run_id2, new_id);
                });
                return;
            }

            if key == Key::F10 {
                running_cb.store(false, Ordering::Relaxed);
                run_id_cb.fetch_add(1, Ordering::Relaxed);
                release_safety();
                return;
            }
        }
        _ => {}
    }) {
        eprintln!("Error: {:?}", err);
    }
}
