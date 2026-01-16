use rand::Rng;
use rdev::{listen, Event, EventType, Key};
use std::{
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
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
    mouse_up(3);
}

fn sleep_range(min: f64, max: f64) {
    let mut rng = rand::thread_rng();
    let secs = rng.gen_range(min..max);
    thread::sleep(Duration::from_secs_f64(secs));
}

#[allow(dead_code)]
fn combo_1_once() {
    // 1
    key_down("s");
    sleep_range(0.05, 0.10);
    click(1);
    key_up("s");

    sleep_range(0.05, 0.06);
    click(3);

    sleep_range(0.6, 0.65);
    click(3);

    sleep_range(0.1, 0.15);
    passive_skill();
}

fn passive_skill() {
    key_down("s");
    mouse_down(3);
    sleep_range(0.6, 0.7);
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
    sleep_range(0.05, 0.10);
    key_down("q");
    key_up("s");
    key_up("q");

    if use_strong_1 {
        strong_skill_1();
    } else {
        strong_skill_2();
    }
}

#[allow(dead_code)]
fn strong_skill_1() {
    key_down("Shift_L");
    mouse_down(1);     
    mouse_down(3); 
    sleep_range(2.2, 2.3);
    mouse_up(1);
    mouse_up(3);
    key_up("Shift_L");

    key_down("f");
    sleep_range(2.65, 2.70);
    key_up("f");

    passive_skill();
}

#[allow(dead_code)]
fn strong_skill_2() {
    // 1
    key_down("Shift_L");
    sleep_range(0.045, 0.050);
    key_down("f");
    sleep_range(1.1,1.15);
    key_up("Shift_L");
    key_up("f");

    // 2
    key_down("Shift_L");
    mouse_down(3); 
    sleep_range(1.8, 1.85);
    key_up("Shift_L");
    mouse_up(3);

    passive_skill();

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

}

#[allow(dead_code)]
fn worker_loop(running: Arc<AtomicBool>) {
    let mut use_strong_1 = true;

    while running.load(Ordering::Relaxed) {
        sleep_range(0.25, 0.30);
       
        combo_1_once();
        sleep_range(0.25, 0.30);

        combo_2_once();
        sleep_range(0.25, 0.30);

        combo_3_once(use_strong_1);
        sleep_range(0.25, 0.30);

        use_strong_1 = !use_strong_1;

    }

    release_safety();
}

fn main() {
    let running = Arc::new(AtomicBool::new(false));
    let busy = Arc::new(AtomicBool::new(false));

    let running_cb = running.clone();
    let busy_cb = busy.clone();

    println!("F9 = Start loop, F10 = Stop");

    if let Err(err) = listen(move |event: Event| match event.event_type {
        EventType::KeyPress(key) => {
            // Start
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
                    // sleep_range(0.25, 0.30);
                    // combo_2_once();
                    // sleep_range(0.25, 0.30);
                    // combo_3_once();
                    // sleep_range(0.25, 0.30);
                    busy2.store(false, Ordering::Relaxed);
                    println!("Done");
                });
                return;
            }

            // Stop
            if key == Key::F10 {
                running_cb.store(false, Ordering::Relaxed);
                release_safety();
                busy_cb.store(false, Ordering::Relaxed);
                println!("Stop (released)");
                return;
            }
        }
        _ => {}
    }) {
        eprintln!("Error: {:?}", err);
    }
}

