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

fn sleep_range(min: f64, max: f64) {
    let mut rng = rand::thread_rng();
    let secs = rng.gen_range(min..max);
    thread::sleep(Duration::from_secs_f64(secs));
}

/// Combo-1:
fn combo_1_once() {
    key_down("s");
    sleep_range(0.02, 0.05);
    click(1);
    key_up("s");

    sleep_range(1.25, 1.35);
    click(3);

    sleep_range(0.45, 0.65);
}

fn release_safety() {
    // กันค้าง (กรณีโดน stop/หรือ error)
    key_up("s");
    mouse_up(3);
}

fn main() {
    let ctrl_down = Arc::new(AtomicBool::new(false));
    let busy = Arc::new(AtomicBool::new(false)); // กันกด Start ซ้ำระหว่างทำคอมโบ

    let ctrl_cb = ctrl_down.clone();
    let busy_cb = busy.clone();

    println!("Ctrl+F11 = Run combo once, Ctrl+F12 = Stop (safety release)");

    if let Err(err) = listen(move |event: Event| {
        match event.event_type {
            EventType::KeyPress(key) => {
                if key == Key::ControlLeft || key == Key::ControlRight {
                    ctrl_cb.store(true, Ordering::Relaxed);
                    return;
                }

                // Start: ทำคอมโบ 1 รอบ
                if key == Key::F11 && ctrl_cb.load(Ordering::Relaxed) {
                    // ถ้ากำลังทำอยู่ ไม่ให้ซ้ำ
                    if busy_cb.swap(true, Ordering::Relaxed) {
                        return;
                    }

                    let busy2 = busy_cb.clone();
                    thread::spawn(move || {
                        println!("Run combo-1...");
                        combo_1_once();
                        release_safety();
                        busy2.store(false, Ordering::Relaxed);
                        println!("Done");
                    });
                    return;
                }

                // Stop: ปล่อยปุ่ม/เมาส์เผื่อค้าง
                if key == Key::F12 && ctrl_cb.load(Ordering::Relaxed) {
                    release_safety();
                    busy_cb.store(false, Ordering::Relaxed);
                    println!("Stop (released)");
                    return;
                }
            }

            EventType::KeyRelease(key) => {
                if key == Key::ControlLeft || key == Key::ControlRight {
                    ctrl_cb.store(false, Ordering::Relaxed);
                    return;
                }
            }

            _ => {}
        }
    }) {
        eprintln!("Error: {:?}", err);
    }
}
