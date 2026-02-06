use rand::Rng;
use rdev::{ listen, Event, EventType, Key };
use std::{
    collections::HashSet,
    process::Command,
    sync::{ atomic::{ AtomicBool, AtomicU64, Ordering }, Arc, Mutex },
    thread,
    time::{ Duration, Instant },
};

fn xdotool(args: &[&str]) {
    let _ = Command::new("xdotool").args(args).status();
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

    pressed_keys: Arc<Mutex<HashSet<String>>>,
    pressed_mouse: Arc<Mutex<HashSet<u8>>>,
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
                self.release_all();
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

    fn key_down(&self, key: &str) {
        {
            let mut set = self.pressed_keys.lock().unwrap();
            set.insert(key.to_string());
        }
        xdotool(&["keydown", key]);
    }

    fn key_up(&self, key: &str) {
        {
            let mut set = self.pressed_keys.lock().unwrap();
            set.remove(key);
        }
        xdotool(&["keyup", key]);
    }

    fn mouse_down(&self, button: u8) {
        {
            let mut set = self.pressed_mouse.lock().unwrap();
            set.insert(button);
        }
        xdotool(&["mousedown", &button.to_string()]);
    }

    fn mouse_up(&self, button: u8) {
        {
            let mut set = self.pressed_mouse.lock().unwrap();
            set.remove(&button);
        }
        xdotool(&["mouseup", &button.to_string()]);
    }

    fn click(&self, button: u8) {
        xdotool(&["click", &button.to_string()]);
    }

    fn tap_key(&self, key: &str) -> bool {
        if self.stop_requested() {
            self.release_all();
            return false;
        }
        self.key_down(key);
        if !self.sleep_interruptible(0.01) {
            return false;
        }
        self.key_up(key);
        true
    }

    fn release_all(&self) {
        let keys: Vec<String> = {
            let mut set = self.pressed_keys.lock().unwrap();
            let v = set.iter().cloned().collect::<Vec<_>>();
            set.clear();
            v
        };
        for k in keys {
            xdotool(&["keyup", &k]);
        }

        let btns: Vec<u8> = {
            let mut set = self.pressed_mouse.lock().unwrap();
            let v = set.iter().cloned().collect::<Vec<_>>();
            set.clear();
            v
        };
        for b in btns {
            xdotool(&["mouseup", &b.to_string()]);
        }

        // xdotool(&["keyup", "Shift_L"]);
        // xdotool(&["mouseup", "1"]);
        // xdotool(&["mouseup", "2"]);
        // xdotool(&["mouseup", "3"]);
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
        after_sleep: f64
    ) -> bool {
        if ctrl.stop_requested() {
            ctrl.release_all();
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
            ctrl.release_all();
            return false;
        }

        if !ctrl.tap_key("z") {
            return false;
        }

        ctrl.release_all();
        true
    }
}

fn passive_skill(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("s");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    ctrl.mouse_down(3);
    if !ctrl.sleep_range(0.45, 0.5) {
        return false;
    }
    ctrl.mouse_up(3);
    ctrl.key_up("s");
    true
}

fn combo_1_once(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("s");
    if !ctrl.sleep_range(0.05, 0.1) {
        return false;
    }
    ctrl.click(1);
    ctrl.key_up("s");

    if !ctrl.sleep_range(0.05, 0.06) {
        return false;
    }
    ctrl.click(3);

    if !ctrl.sleep_range(0.5, 0.55) {
        return false;
    }
    ctrl.click(3);

    if !ctrl.sleep_range(0.1, 0.2) {
        return false;
    }
    if !passive_skill(ctrl) {
        return false;
    }

    ctrl.release_all();
    true
}

fn combo_2_once(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("s");
    if !ctrl.sleep_range(0.05, 0.1) {
        return false;
    }

    ctrl.key_down("f");
    ctrl.key_up("f");
    ctrl.key_up("s");

    if !ctrl.sleep_range(0.6, 0.65) {
        return false;
    }
    ctrl.click(3);

    if !ctrl.sleep_range(1.1, 1.15) {
        return false;
    }
    ctrl.click(3);

    if !ctrl.sleep_range(0.75, 0.8) {
        return false;
    }
    ctrl.click(3);

    if !ctrl.sleep_range(0.75, 0.8) {
        return false;
    }
    ctrl.click(3);

    if !ctrl.sleep_range(0.85, 0.9) {
        return false;
    }
    if !passive_skill(ctrl) {
        return false;
    }

    ctrl.release_all();
    true
}

fn strong_skill_1(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("Shift_L");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    ctrl.mouse_down(1);
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    ctrl.mouse_down(3);
    if !ctrl.sleep_range(2.2, 2.3) {
        return false;
    }

    ctrl.key_up("Shift_L");
    ctrl.mouse_up(1);
    ctrl.mouse_up(3);

    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("f");
    if !ctrl.sleep_range(2.0, 2.05) {
        return false;
    }
    ctrl.key_up("f");

    if !passive_skill(ctrl) {
        return false;
    }
    ctrl.release_all();
    true
}

fn strong_skill_2(ctrl: &Ctrl) -> bool {
    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("Shift_L");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    ctrl.key_down("f");
    if !ctrl.sleep_range(1.1, 1.15) {
        return false;
    }
    ctrl.key_up("Shift_L");
    ctrl.key_up("f");

    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("Shift_L");
    ctrl.mouse_down(3);
    if !ctrl.sleep_range(1.65, 1.7) {
        return false;
    }
    ctrl.key_up("Shift_L");
    ctrl.mouse_up(3);

    if !passive_skill(ctrl) {
        return false;
    }

    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("s");
    if !ctrl.sleep_range(0.045, 0.05) {
        return false;
    }
    ctrl.mouse_down(1);
    ctrl.mouse_down(3);
    if !ctrl.sleep_range(1.2, 1.25) {
        return false;
    }
    ctrl.mouse_up(1);
    ctrl.mouse_up(3);
    if !ctrl.sleep_range(0.045, 0.05) {
        return false;
    }
    ctrl.key_up("s");

    if !ctrl.sleep_range(0.5, 0.6) {
        return false;
    }
    ctrl.mouse_up(3);

    ctrl.release_all();
    true
}

fn combo_3_once(ctrl: &Ctrl, use_strong_1: bool) -> bool {
    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("s");
    ctrl.key_down("c");
    if !ctrl.sleep_range(0.05, 0.1) {
        return false;
    }
    ctrl.key_up("s");
    ctrl.key_up("c");

    if !ctrl.sleep_range(1.05, 1.1) {
        return false;
    }

    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.mouse_down(1);
    ctrl.mouse_down(3);
    if !ctrl.sleep_range(0.01, 0.05) {
        return false;
    }
    ctrl.mouse_up(1);
    ctrl.mouse_up(3);

    if !ctrl.sleep_range(0.65, 0.7) {
        return false;
    }

    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    ctrl.key_down("s");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    ctrl.key_down("q");
    ctrl.key_up("s");
    if !ctrl.sleep_range(0.01, 0.02) {
        return false;
    }
    ctrl.key_up("q");

    if ctrl.stop_requested() {
        ctrl.release_all();
        return false;
    }

    let ok = if use_strong_1 { strong_skill_1(ctrl) } else { strong_skill_2(ctrl) };

    if !ok {
        return false;
    }

    ctrl.release_all();
    true
}

fn worker_loop(ctrl: Ctrl) {
    let start = Instant::now();
    let mut round: u64 = 0;
    let mut buffs = BuffCooldown::new();

    if !buffs.buff_once(&ctrl) {
        ctrl.release_all();
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

        ctrl.release_all();
        if !ctrl.sleep_range(0.25, 0.3) {
            break;
        }

        round += 1;
        log_elapsed(start, &format!("Round #{round} done"));
    }

    ctrl.release_all();
    log_elapsed(start, "Stopped");
}

fn main() {
    let running = Arc::new(AtomicBool::new(false));
    let run_id = Arc::new(AtomicU64::new(0));

    let pressed_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
    let pressed_mouse = Arc::new(Mutex::new(HashSet::<u8>::new()));

    let running_cb = running.clone();
    let run_id_cb = run_id.clone();

    let pk_cb = pressed_keys.clone();
    let pm_cb = pressed_mouse.clone();

    println!("F9 = Start loop, F10 = Stop");

    if
        let Err(err) = listen(move |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    if key == Key::F9 {
                        let new_id = run_id_cb.fetch_add(1, Ordering::Relaxed) + 1;
                        running_cb.store(true, Ordering::Relaxed);

                        let ctrl = Ctrl {
                            running: running_cb.clone(),
                            run_id: run_id_cb.clone(),
                            my_id: new_id,
                            pressed_keys: pk_cb.clone(),
                            pressed_mouse: pm_cb.clone(),
                        };
                        ctrl.release_all();

                        thread::spawn(move || worker_loop(ctrl));
                        return;
                    }

                    if key == Key::F10 {
                        running_cb.store(false, Ordering::Relaxed);
                        run_id_cb.fetch_add(1, Ordering::Relaxed);

                        let ctrl = Ctrl {
                            running: running_cb.clone(),
                            run_id: run_id_cb.clone(),
                            my_id: run_id_cb.load(Ordering::Relaxed),
                            pressed_keys: pk_cb.clone(),
                            pressed_mouse: pm_cb.clone(),
                        };
                        ctrl.release_all();
                        return;
                    }
                }
                _ => {}
            }
        })
    {
        eprintln!("Error: {:?}", err);
    }
}
