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

    sleep_range(1.20, 1.25);
    click(3);

    sleep_range(0.80, 0.85);
    click(3);

    sleep_range(0.80, 0.85);
    click(3);

    sleep_range(0.90, 0.95);
    passive_skill();
}

#[allow(dead_code)]
fn combo_3_once() {
    key_down("s");
    sleep_range(0.05, 0.10);
    key_down("c");
    key_up("s");
    key_up("c");

    sleep_range(1.65, 1.70);

    mouse_down(1);
    mouse_down(3);
    sleep_range(0.01, 0.05);
    mouse_up(1);
    mouse_up(3);

    sleep_range(0.65, 0.7);

    key_down("s");
    sleep_range(0.05, 0.10);
    key_down("q");
    key_up("s");
    key_up("q");

    // strong_skill_1()
    strong_skill_2()
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
    sleep_range(2.2, 2.3);
    key_up("f");

    passive_skill();
}

#[allow(dead_code)]
fn strong_skill_2() {
    key_down("Shift_L");
    sleep_range(0.045, 0.050);
    key_down("f");
    sleep_range(1.1,1.15);
    key_up("Shift_L");
    key_up("f");

    key_down("Shift_L");
    mouse_down(3); 
    sleep_range(1.2, 1.25);
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

fn main() {
    let busy = Arc::new(AtomicBool::new(false));
    let busy_cb = busy.clone();

    println!("F9 = Run combo once, F10 = Stop (safety release)");

    if let Err(err) = listen(move |event: Event| match event.event_type {
        EventType::KeyPress(key) => {
            if key == Key::F9 {
                if busy_cb.swap(true, Ordering::Relaxed) {
                    return;
                }

                let busy2 = busy_cb.clone();
                thread::spawn(move || {
                    // combo_1_once();
                    // sleep_range(0.2, 0.25);
                    // combo_2_once();
                    strong_skill_2();

                    busy2.store(false, Ordering::Relaxed);
                    println!("Done");
                });
                return;
            }

            if key == Key::F10 {
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
