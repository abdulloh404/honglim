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
    sleep_range(0.02, 0.25);
    click(1);
    key_up("s");

    sleep_range(0.02, 0.25);
    click(3);

    sleep_range(0.5, 0.55);
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
    sleep_range(0.02, 0.03);
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
                    println!("Run combo-1...");
                    combo_1_once();

                    sleep_range(0.4, 0.45);

                    println!("Run combo-2...");
                    combo_2_once();
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
