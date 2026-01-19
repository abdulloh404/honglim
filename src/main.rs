use rand::Rng;
use rdev::{listen, Event, EventType, Key};
use std::{
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
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
    key_up("Shift_L");
    mouse_up(1);
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

fn sleep_range(min: f64, max: f64) {
    let mut rng = rand::thread_rng();
    let secs = rng.gen_range(min..max);
    thread::sleep(Duration::from_secs_f64(secs));
}

fn sleep_exact(secs: f64) {
    thread::sleep(Duration::from_secs_f64(secs));
}

fn tap_key(key: &str) {
    key_down(key);
    sleep_exact(0.01);
    key_up(key);
}

#[derive(Debug)]
struct BuffCooldown {
    last_q: Option<Instant>, // 60s
    last_2: Option<Instant>, // 60s
    last_3: Option<Instant>, // 60s
    last_4: Option<Instant>, // 180s
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

    fn press_if_ready(key: &str, last: &mut Option<Instant>, cd: Duration, after_sleep: f64) {
        if Self::ready(*last, cd) {
            tap_key(key);
            *last = Some(Instant::now());
            sleep_exact(after_sleep);
        }
    }

    fn buff_once(&mut self) {
        Self::press_if_ready("q", &mut self.last_q, Duration::from_secs(60), 0.75);
        Self::press_if_ready("2", &mut self.last_2, Duration::from_secs(60), 0.95);
        Self::press_if_ready("3", &mut self.last_3, Duration::from_secs(60), 0.75);
        Self::press_if_ready("4", &mut self.last_4, Duration::from_secs(180), 0.95);

        release_safety();
    }
}

#[allow(dead_code)]
fn combo_1_once() {
    key_down("s");
    sleep_range(0.05, 0.10);
    click(1);
    key_up("s");

    sleep_range(0.05, 0.06);
    click(3);

    sleep_range(0.5, 0.55);
    click(3);

    sleep_range(0.1, 0.2);
    passive_skill();
    release_safety();
}

fn passive_skill() {
    key_down("s");
    sleep_range(0.01, 0.02);
    mouse_down(3);
    sleep_range(0.45, 0.50);
    mouse_up(3);
    key_up("s");
}

#[allow(dead_code)]
fn combo_2_once() {
    key_down("s");
    sleep_range(0.05, 0.10);
    key_down("f");
    key_up("f");
    key_up("s");

    sleep_range(0.60, 0.65);
    click(3);

    sleep_range(1.10, 1.15);
    click(3);

    sleep_range(0.75, 0.80);
    click(3);

    sleep_range(0.75, 0.80);
    click(3);

    sleep_range(0.85, 0.90);
    passive_skill();
    release_safety();
}

#[allow(dead_code)]
fn combo_3_once(use_strong_1: bool) {
    // 1
    key_down("s");
    sleep_range(0.05, 0.10);
    key_down("c");
    key_up("s");
    key_up("c");

    sleep_range(1.05, 1.10);

    // 2
    mouse_down(1);
    mouse_down(3);
    sleep_range(0.01, 0.05);
    mouse_up(1);
    mouse_up(3);

    sleep_range(0.65, 0.7);

    // 3
    key_down("s");
    sleep_range(0.01, 0.02);
    key_down("q");
    key_up("s");
    sleep_range(0.01, 0.02);
    key_up("q");

    if use_strong_1 {
        strong_skill_1();
    } else {
        strong_skill_2();
    }

    release_safety();
}

#[allow(dead_code)]
fn strong_skill_1() {
    // 1
    key_down("Shift_L");
    sleep_range(0.01, 0.02);
    mouse_down(1);
    sleep_range(0.01, 0.02);
    mouse_down(3);
    sleep_range(2.2, 2.3);

    key_up("Shift_L");
    mouse_up(1);
    mouse_up(3);

    // 2
    key_down("f");
    sleep_range(2.0, 2.05);
    key_up("f");

    passive_skill();
    release_safety();
}

#[allow(dead_code)]
fn strong_skill_2() {
    // 1
    key_down("Shift_L");
    sleep_range(0.01, 0.02);
    key_down("f");
    sleep_range(1.1, 1.15);
    key_up("Shift_L");
    key_up("f");

    // 2
    key_down("Shift_L");
    mouse_down(3);
    sleep_range(1.65, 1.70);
    key_up("Shift_L");
    mouse_up(3);

    passive_skill();

    // 3
    key_down("s");
    sleep_range(0.045, 0.050);
    mouse_down(1);
    mouse_down(3);
    sleep_range(1.2, 1.25);
    mouse_up(1);
    mouse_up(3);
    sleep_range(0.045, 0.050);
    key_up("s");
    
    sleep_range(0.5, 0.6);
    mouse_up(3);

    release_safety();
}

#[allow(dead_code)]
fn worker_loop(running: Arc<AtomicBool>) {
    let start = Instant::now();
    let mut round: u64 = 0;

    let mut buffs = BuffCooldown::new();

    buffs.buff_once();
    round += 1;
    log_elapsed(start, &format!("Round #{round} (pre-buff)"));

    while running.load(Ordering::Relaxed) {
        sleep_range(0.25, 0.30);
       
        // round 1
        combo_1_once();
        sleep_range(0.25, 0.30);

        combo_2_once();
        sleep_range(0.25, 0.30);

        combo_3_once(true);
        sleep_range(0.6, 0.65);

        // round 2
        combo_1_once();
        sleep_range(0.25, 0.30);

        combo_2_once();
        sleep_range(0.25, 0.30);

        combo_3_once(false);
        sleep_range(0.25, 0.30);

        buffs.buff_once();

        release_safety();
        sleep_range(0.25, 0.30);

        round += 1;
        log_elapsed(start, &format!("Round #{round} done"));
    }

    release_safety();
    log_elapsed(start, "Stopped");
}

fn main() {
    let running = Arc::new(AtomicBool::new(false));
    let busy = Arc::new(AtomicBool::new(false));

    let running_cb = running.clone();
    let busy_cb = busy.clone();

    println!("F9 = Start loop, F10 = Stop");

    if let Err(err) = listen(move |event: Event| match event.event_type {
        EventType::KeyPress(key) => {
            if key == Key::F9 {
                if busy_cb.swap(true, Ordering::Relaxed) {
                    return;
                }

                running_cb.store(true, Ordering::Relaxed);

                let running2 = running_cb.clone();
                let busy2 = busy_cb.clone();
                thread::spawn(move || {
                    worker_loop(running2);
                   
                    // combo_1_once();
                    // sleep_range(0.30, 0.35);
                    // combo_2_once();
                    // sleep_range(0.30, 0.35);
                    // combo_3_once(true);
                    // sleep_range(0.30, 0.35);
                    // strong_skill_1();
                    // strong_skill_2();
                    busy2.store(false, Ordering::Relaxed);
                    // println!("Done");
                });
                return;
            }

            if key == Key::F10 {
                running_cb.store(false, Ordering::Relaxed);
                release_safety();
                busy_cb.store(false, Ordering::Relaxed);
                // println!("Stop");
                return;
            }
        }
        _ => {}
    }) {
        eprintln!("Error: {:?}", err);
    }
}