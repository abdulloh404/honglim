use rand::Rng;
use rdev::{ listen, Event, EventType, Key };
use std::{
    collections::{ HashMap, HashSet },
    process::Command,
    sync::{ atomic::{ AtomicBool, AtomicU64, Ordering }, Arc, Mutex },
    thread,
    time::{ Duration, Instant },
};

fn run_xdotool_command(arguments: &[&str]) {
    let _ = Command::new("xdotool").args(arguments).status();
}

fn format_elapsed_time_mm_ss(program_start_time: Instant) -> String {
    let elapsed_seconds = program_start_time.elapsed().as_secs();
    let minutes = elapsed_seconds / 60;
    let seconds = elapsed_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

fn log_with_elapsed_time(program_start_time: Instant, message: &str) {
    println!("{} | {}", message, format_elapsed_time_mm_ss(program_start_time));
}

fn is_user_manual_pause_key(key: Key) -> bool {
    matches!(
        key,
        Key::KeyQ |
            Key::KeyW |
            Key::KeyE |
            Key::KeyA |
            Key::KeyS |
            Key::KeyD |
            Key::KeyF |
            Key::KeyZ |
            Key::KeyX |
            Key::KeyC |
            Key::Space |
            Key::KeyV |
            Key::ShiftLeft
    )
}

fn xdotool_key_name_from_rdev_key(key: Key) -> Option<&'static str> {
    match key {
        Key::KeyQ => Some("q"),
        Key::KeyW => Some("w"),
        Key::KeyE => Some("e"),
        Key::KeyA => Some("a"),
        Key::KeyS => Some("s"),
        Key::KeyD => Some("d"),
        Key::KeyF => Some("f"),
        Key::KeyZ => Some("z"),
        Key::KeyX => Some("x"),
        Key::KeyC => Some("c"),
        Key::KeyV => Some("v"),
        Key::Space => Some("space"),
        Key::ShiftLeft => Some("Shift_L"),
        _ => None,
    }
}

#[derive(Clone)]
struct ControllerContext {
    is_worker_running_flag: Arc<AtomicBool>,
    active_worker_run_id: Arc<AtomicU64>,
    this_worker_run_id: u64,

    system_pressed_keyboard_keys: Arc<Mutex<HashSet<String>>>,
    system_pressed_mouse_buttons: Arc<Mutex<HashSet<u8>>>,

    user_pressed_pause_keyboard_keys: Arc<Mutex<HashSet<Key>>>,
    last_user_input_change_time: Arc<Mutex<Instant>>,

    recent_injected_key_times: Arc<Mutex<HashMap<String, Instant>>>,

    should_skip_next_passive_skill_once: Arc<AtomicBool>,
    should_skip_rest_of_current_combo_once: Arc<AtomicBool>,
}

impl ControllerContext {
    fn is_stop_requested(&self) -> bool {
        !self.is_worker_running_flag.load(Ordering::Relaxed) ||
            self.active_worker_run_id.load(Ordering::Relaxed) != self.this_worker_run_id
    }

    fn is_user_currently_requesting_pause(&self) -> bool {
        !self.user_pressed_pause_keyboard_keys.lock().unwrap().is_empty()
    }

    fn mark_should_skip_next_passive_skill_once(&self) {
        self.should_skip_next_passive_skill_once.store(true, Ordering::Relaxed);
    }

    fn take_should_skip_next_passive_skill_once(&self) -> bool {
        self.should_skip_next_passive_skill_once.swap(false, Ordering::Relaxed)
    }

    fn mark_should_skip_rest_of_current_combo_once(&self) {
        self.should_skip_rest_of_current_combo_once.store(true, Ordering::Relaxed);
    }

    fn take_should_skip_rest_of_current_combo_once(&self) -> bool {
        self.should_skip_rest_of_current_combo_once.swap(false, Ordering::Relaxed)
    }

    fn wait_until_user_is_idle_for_seconds(&self, required_idle_seconds: f64) -> bool {
        let required_idle_duration = Duration::from_secs_f64(required_idle_seconds);
        let sleep_tick = Duration::from_millis(5);

        let mut saw_pause_during_wait = false;

        loop {
            if self.is_stop_requested() {
                self.release_all_system_inputs();
                return false;
            }

            if self.is_user_currently_requesting_pause() {
                saw_pause_during_wait = true;
                self.release_all_system_inputs();
                thread::sleep(sleep_tick);
                continue;
            }

            let last_change_time = *self.last_user_input_change_time.lock().unwrap();
            if last_change_time.elapsed() >= required_idle_duration {
                if saw_pause_during_wait {
                    self.mark_should_skip_next_passive_skill_once();
                    self.mark_should_skip_rest_of_current_combo_once();
                }
                return true;
            }

            thread::sleep(sleep_tick);
        }
    }

    fn mark_recent_injected_key_time(&self, key: &str) {
        self.recent_injected_key_times.lock().unwrap().insert(key.to_string(), Instant::now());
    }

    fn sleep_seconds_interruptible(&self, seconds: f64) -> bool {
        if seconds <= 0.0 {
            return !self.is_stop_requested();
        }

        let sleep_tick = Duration::from_millis(5);
        let mut remaining = Duration::from_secs_f64(seconds);

        while remaining > Duration::ZERO {
            if self.is_stop_requested() {
                self.release_all_system_inputs();
                return false;
            }

            if self.is_user_currently_requesting_pause() {
                self.release_all_system_inputs();
                if !self.wait_until_user_is_idle_for_seconds(0.1) {
                    return false;
                }
                self.mark_should_skip_next_passive_skill_once();
                self.mark_should_skip_rest_of_current_combo_once();
                return true;
            }

            let this_tick = std::cmp::min(sleep_tick, remaining);
            thread::sleep(this_tick);
            remaining = remaining.saturating_sub(this_tick);
        }

        true
    }

    fn sleep_random_range_seconds_interruptible(&self, min_seconds: f64, max_seconds: f64) -> bool {
        let mut rng = rand::thread_rng();
        let chosen_seconds = rng.gen_range(min_seconds..max_seconds);
        self.sleep_seconds_interruptible(chosen_seconds)
    }

    fn system_key_down(&self, key: &str) {
        self.mark_recent_injected_key_time(key);
        {
            let mut pressed = self.system_pressed_keyboard_keys.lock().unwrap();
            pressed.insert(key.to_string());
        }
        run_xdotool_command(&["keydown", key]);
    }

    fn system_key_up(&self, key: &str) {
        self.mark_recent_injected_key_time(key);
        {
            let mut pressed = self.system_pressed_keyboard_keys.lock().unwrap();
            pressed.remove(key);
        }
        run_xdotool_command(&["keyup", key]);
    }

    fn system_mouse_button_down(&self, button: u8) {
        {
            let mut pressed = self.system_pressed_mouse_buttons.lock().unwrap();
            pressed.insert(button);
        }
        run_xdotool_command(&["mousedown", &button.to_string()]);
    }

    fn system_mouse_button_up(&self, button: u8) {
        {
            let mut pressed = self.system_pressed_mouse_buttons.lock().unwrap();
            pressed.remove(&button);
        }
        run_xdotool_command(&["mouseup", &button.to_string()]);
    }

    fn system_click_mouse_button(&self, button: u8) {
        run_xdotool_command(&["click", &button.to_string()]);
    }

    fn system_tap_key(&self, key: &str) -> bool {
        if self.is_stop_requested() {
            self.release_all_system_inputs();
            return false;
        }

        if !self.wait_until_user_is_idle_for_seconds(0.1) {
            return false;
        }

        if self.take_should_skip_rest_of_current_combo_once() {
            self.release_all_system_inputs();
            return true;
        }

        self.system_key_down(key);
        if !self.sleep_seconds_interruptible(0.01) {
            return false;
        }

        if self.take_should_skip_rest_of_current_combo_once() {
            self.release_all_system_inputs();
            return true;
        }

        self.system_key_up(key);
        true
    }

    fn release_all_system_inputs(&self) {
        let pressed_keys_snapshot: Vec<String> = {
            let mut pressed = self.system_pressed_keyboard_keys.lock().unwrap();
            let snapshot = pressed.iter().cloned().collect::<Vec<_>>();
            pressed.clear();
            snapshot
        };
        for key in pressed_keys_snapshot {
            self.mark_recent_injected_key_time(&key);
            run_xdotool_command(&["keyup", &key]);
        }

        let pressed_mouse_snapshot: Vec<u8> = {
            let mut pressed = self.system_pressed_mouse_buttons.lock().unwrap();
            let snapshot = pressed.iter().cloned().collect::<Vec<_>>();
            pressed.clear();
            snapshot
        };
        for button in pressed_mouse_snapshot {
            run_xdotool_command(&["mouseup", &button.to_string()]);
        }
    }
}

fn should_abort_remaining_actions_of_current_combo(controller: &ControllerContext) -> bool {
    if controller.take_should_skip_rest_of_current_combo_once() {
        controller.release_all_system_inputs();
        return true;
    }
    false
}

#[derive(Debug)]
struct BuffCooldownTracker {
    last_pressed_q: Option<Instant>,
    last_pressed_1: Option<Instant>,
    last_pressed_2: Option<Instant>,
    last_pressed_3: Option<Instant>,
    // last_pressed_z: Option<Instant>,
}

impl BuffCooldownTracker {
    fn new() -> Self {
        Self {
            last_pressed_q: None,
            last_pressed_1: None,
            last_pressed_2: None,
            last_pressed_3: None,
            // last_pressed_z: None,
        }
    }

    fn is_ready(last_pressed: Option<Instant>, cooldown: Duration) -> bool {
        match last_pressed {
            None => true,
            Some(time) => time.elapsed() >= cooldown,
        }
    }

    fn press_key_if_ready(
        controller: &ControllerContext,
        key: &str,
        last_pressed: &mut Option<Instant>,
        cooldown: Duration,
        after_sleep_seconds: f64
    ) -> bool {
        if controller.is_stop_requested() {
            controller.release_all_system_inputs();
            return false;
        }

        if !controller.wait_until_user_is_idle_for_seconds(0.1) {
            return false;
        }

        if should_abort_remaining_actions_of_current_combo(controller) {
            return true;
        }

        if Self::is_ready(*last_pressed, cooldown) {
            if !controller.system_tap_key(key) {
                return false;
            }

            if should_abort_remaining_actions_of_current_combo(controller) {
                return true;
            }

            *last_pressed = Some(Instant::now());

            if !controller.sleep_seconds_interruptible(after_sleep_seconds) {
                return false;
            }

            if should_abort_remaining_actions_of_current_combo(controller) {
                return true;
            }
        }

        true
    }

    fn apply_buffs_once(&mut self, controller: &ControllerContext) -> bool {
        if
            !Self::press_key_if_ready(
                controller,
                "q",
                &mut self.last_pressed_q,
                Duration::from_secs(120),
                0.75
            )
        {
            return false;
        }

        if
            !Self::press_key_if_ready(
                controller,
                "1",
                &mut self.last_pressed_1,
                Duration::from_secs(600),
                0.95
            )
        {
            return false;
        }

        if
            !Self::press_key_if_ready(
                controller,
                "2",
                &mut self.last_pressed_2,
                Duration::from_secs(50),
                0.75
            )
        {
            return false;
        }

        if
            !Self::press_key_if_ready(
                controller,
                "3",
                &mut self.last_pressed_3,
                Duration::from_secs(180),
                1.2
            )
        {
            return false;
        }

        // if
        //     !Self::press_key_if_ready(
        //         controller,
        //         "z",
        //         &mut self.last_pressed_z,
        //         Duration::from_secs(180),
        //         1.3
        //     )
        // {
        //     return false;
        // }

        if !controller.system_tap_key("z") {
            return false;
        }

        if !controller.sleep_seconds_interruptible(1.3) {
            return false;
        }

        controller.release_all_system_inputs();
        true
    }
}

fn perform_passive_skill(controller: &ControllerContext) -> bool {
    if controller.take_should_skip_next_passive_skill_once() {
        controller.release_all_system_inputs();
        return true;
    }

    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(0.45, 0.5) {
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_mouse_button_up(3);
    controller.system_key_up("s");
    true
}

fn perform_combo_1_once(controller: &ControllerContext) -> bool {
    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 1 - S + Rmb
    controller.system_key_down("s");

    if !controller.sleep_random_range_seconds_interruptible(0.1, 0.2) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_click_mouse_button(1);

    if !controller.sleep_random_range_seconds_interruptible(0.1, 0.2) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("s");

    if !controller.sleep_random_range_seconds_interruptible(0.05, 0.06) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.5, 0.55) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.1, 0.2) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    if !perform_passive_skill(controller) {
        return false;
    }

    controller.release_all_system_inputs();
    true
}

fn perform_combo_2_once(controller: &ControllerContext) -> bool {
    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 1 - S + F
    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.05, 0.07) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_down("f");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("f");
    controller.system_key_up("s");

    if !controller.sleep_random_range_seconds_interruptible(0.6, 0.65) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(1.1, 1.15) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.75, 0.8) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.75, 0.8) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.85, 0.9) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    if !perform_passive_skill(controller) {
        return false;
    }

    controller.release_all_system_inputs();
    true
}

fn perform_combo_3_once(controller: &ControllerContext, use_strong_skill_1: bool) -> bool {
    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 1 - S + C
    controller.system_key_down("s");
    controller.system_key_down("c");
    if !controller.sleep_random_range_seconds_interruptible(0.05, 0.1) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("s");
    controller.system_key_up("c");

    if !controller.sleep_random_range_seconds_interruptible(1.05, 1.1) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 2 - Rmb + Lmb
    controller.system_mouse_button_down(1);
    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.05) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_mouse_button_up(1);
    controller.system_mouse_button_up(3);

    if !controller.sleep_random_range_seconds_interruptible(0.65, 0.7) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 2 - S + Q
    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_down("q");
    controller.system_key_up("s");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("q");

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    let ok = if use_strong_skill_1 {
        perform_strong_skill_1(controller)
    } else {
        perform_strong_skill_2(controller)
    };

    if !ok {
        return false;
    }

    controller.release_all_system_inputs();
    true
}

fn perform_strong_skill_1(controller: &ControllerContext) -> bool {
    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 1 - Shift + Rmb & Lmb
    controller.system_key_down("Shift_L");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_mouse_button_down(1);
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(2.2, 2.3) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("Shift_L");
    controller.system_mouse_button_up(1);
    controller.system_mouse_button_up(3);

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    if !perform_passive_skill(controller) {
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 2 - F
    controller.system_key_down("f");
    if !controller.sleep_random_range_seconds_interruptible(2.0, 2.05) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("f");

    controller.release_all_system_inputs();
    true
}

fn perform_strong_skill_2(controller: &ControllerContext) -> bool {
    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 1 - Shift + F
    controller.system_key_down("Shift_L");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_down("f");
    if !controller.sleep_random_range_seconds_interruptible(1.1, 1.15) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("Shift_L");
    controller.system_key_up("f");

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 2 - Shift + Rmb
    controller.system_key_down("Shift_L");
    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(1.7, 1.75) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("Shift_L");
    controller.system_mouse_button_up(3);

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    if !perform_passive_skill(controller) {
        return false;
    }

    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    // 3 - S + Rmb & Lmb
    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.045, 0.05) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_mouse_button_down(1);
    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(1.2, 1.25) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_mouse_button_up(1);
    controller.system_mouse_button_up(3);
    if !controller.sleep_random_range_seconds_interruptible(0.045, 0.05) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_key_up("s");

    if !controller.sleep_random_range_seconds_interruptible(0.5, 0.6) {
        return false;
    }
    if should_abort_remaining_actions_of_current_combo(controller) {
        return true;
    }

    controller.system_mouse_button_up(3);

    controller.release_all_system_inputs();
    true
}

fn run_skill_worker_loop(controller: ControllerContext) {
    let worker_log_start_time = Instant::now();
    let mut completed_rounds: u64 = 0;
    let mut buff_cooldown_tracker = BuffCooldownTracker::new();

    if !buff_cooldown_tracker.apply_buffs_once(&controller) {
        controller.release_all_system_inputs();
        return;
    }

    completed_rounds += 1;
    log_with_elapsed_time(worker_log_start_time, &format!("Round #{completed_rounds} (pre-buff)"));

    while !controller.is_stop_requested() {
        if !controller.sleep_random_range_seconds_interruptible(0.1, 0.15) {
            break;
        }

        if !perform_combo_1_once(&controller) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.1, 0.15) {
            break;
        }

        if !perform_combo_2_once(&controller) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.3, 0.35) {
            break;
        }

        if !perform_combo_3_once(&controller, true) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.1, 0.15) {
            break;
        }

        if !perform_combo_1_once(&controller) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.1, 0.15) {
            break;
        }

        if !perform_combo_2_once(&controller) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.3, 0.35) {
            break;
        }

        if !perform_combo_3_once(&controller, false) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.1, 0.15) {
            break;
        }

        if !buff_cooldown_tracker.apply_buffs_once(&controller) {
            break;
        }

        controller.release_all_system_inputs();
        if !controller.sleep_random_range_seconds_interruptible(0.1, 0.15) {
            break;
        }

        completed_rounds += 1;
        log_with_elapsed_time(worker_log_start_time, &format!("Round #{completed_rounds} done"));
    }

    controller.release_all_system_inputs();
    log_with_elapsed_time(worker_log_start_time, "Stopped");
}

fn is_injected_pause_key_event(
    system_pressed_keyboard_keys: &Arc<Mutex<HashSet<String>>>,
    recent_injected_key_times: &Arc<Mutex<HashMap<String, Instant>>>,
    key: Key
) -> bool {
    let Some(key_name) = xdotool_key_name_from_rdev_key(key) else {
        return false;
    };

    if system_pressed_keyboard_keys.lock().unwrap().contains(key_name) {
        return true;
    }

    if
        let Some(last_injected_time) = recent_injected_key_times
            .lock()
            .unwrap()
            .get(key_name)
            .cloned()
    {
        if last_injected_time.elapsed() < Duration::from_millis(120) {
            return true;
        }
    }

    false
}

fn main() {
    let is_worker_running_flag = Arc::new(AtomicBool::new(false));
    let active_worker_run_id = Arc::new(AtomicU64::new(0));

    let system_pressed_keyboard_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
    let system_pressed_mouse_buttons = Arc::new(Mutex::new(HashSet::<u8>::new()));

    let user_pressed_pause_keyboard_keys = Arc::new(Mutex::new(HashSet::<Key>::new()));
    let last_user_input_change_time = Arc::new(
        Mutex::new(Instant::now() - Duration::from_secs(60))
    );

    let recent_injected_key_times = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));

    let should_skip_next_passive_skill_once = Arc::new(AtomicBool::new(false));
    let should_skip_rest_of_current_combo_once = Arc::new(AtomicBool::new(false));

    let is_worker_running_flag_for_listener = is_worker_running_flag.clone();
    let active_worker_run_id_for_listener = active_worker_run_id.clone();

    let system_pressed_keyboard_keys_for_listener = system_pressed_keyboard_keys.clone();
    let system_pressed_mouse_buttons_for_listener = system_pressed_mouse_buttons.clone();

    let user_pressed_pause_keyboard_keys_for_listener = user_pressed_pause_keyboard_keys.clone();
    let last_user_input_change_time_for_listener = last_user_input_change_time.clone();

    let recent_injected_key_times_for_listener = recent_injected_key_times.clone();

    let should_skip_next_passive_skill_once_for_listener =
        should_skip_next_passive_skill_once.clone();
    let should_skip_rest_of_current_combo_once_for_listener =
        should_skip_rest_of_current_combo_once.clone();

    println!("F9 = Start loop, F10 = Stop");

    if
        let Err(error) = listen(move |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    if key == Key::F9 {
                        let new_worker_run_id =
                            active_worker_run_id_for_listener.fetch_add(1, Ordering::Relaxed) + 1;
                        is_worker_running_flag_for_listener.store(true, Ordering::Relaxed);

                        let controller = ControllerContext {
                            is_worker_running_flag: is_worker_running_flag_for_listener.clone(),
                            active_worker_run_id: active_worker_run_id_for_listener.clone(),
                            this_worker_run_id: new_worker_run_id,

                            system_pressed_keyboard_keys: system_pressed_keyboard_keys_for_listener.clone(),
                            system_pressed_mouse_buttons: system_pressed_mouse_buttons_for_listener.clone(),

                            user_pressed_pause_keyboard_keys: user_pressed_pause_keyboard_keys_for_listener.clone(),
                            last_user_input_change_time: last_user_input_change_time_for_listener.clone(),

                            recent_injected_key_times: recent_injected_key_times_for_listener.clone(),

                            should_skip_next_passive_skill_once: should_skip_next_passive_skill_once_for_listener.clone(),
                            should_skip_rest_of_current_combo_once: should_skip_rest_of_current_combo_once_for_listener.clone(),
                        };

                        controller.release_all_system_inputs();
                        thread::spawn(move || run_skill_worker_loop(controller));
                        return;
                    }

                    if key == Key::F10 {
                        is_worker_running_flag_for_listener.store(false, Ordering::Relaxed);
                        active_worker_run_id_for_listener.fetch_add(1, Ordering::Relaxed);

                        let controller = ControllerContext {
                            is_worker_running_flag: is_worker_running_flag_for_listener.clone(),
                            active_worker_run_id: active_worker_run_id_for_listener.clone(),
                            this_worker_run_id: active_worker_run_id_for_listener.load(
                                Ordering::Relaxed
                            ),

                            system_pressed_keyboard_keys: system_pressed_keyboard_keys_for_listener.clone(),
                            system_pressed_mouse_buttons: system_pressed_mouse_buttons_for_listener.clone(),

                            user_pressed_pause_keyboard_keys: user_pressed_pause_keyboard_keys_for_listener.clone(),
                            last_user_input_change_time: last_user_input_change_time_for_listener.clone(),

                            recent_injected_key_times: recent_injected_key_times_for_listener.clone(),

                            should_skip_next_passive_skill_once: should_skip_next_passive_skill_once_for_listener.clone(),
                            should_skip_rest_of_current_combo_once: should_skip_rest_of_current_combo_once_for_listener.clone(),
                        };

                        controller.release_all_system_inputs();
                        return;
                    }

                    if
                        is_user_manual_pause_key(key) &&
                        !is_injected_pause_key_event(
                            &system_pressed_keyboard_keys_for_listener,
                            &recent_injected_key_times_for_listener,
                            key
                        )
                    {
                        user_pressed_pause_keyboard_keys_for_listener.lock().unwrap().insert(key);
                        *last_user_input_change_time_for_listener.lock().unwrap() = Instant::now();
                    }
                }

                EventType::KeyRelease(key) => {
                    if
                        is_user_manual_pause_key(key) &&
                        !is_injected_pause_key_event(
                            &system_pressed_keyboard_keys_for_listener,
                            &recent_injected_key_times_for_listener,
                            key
                        )
                    {
                        user_pressed_pause_keyboard_keys_for_listener.lock().unwrap().remove(&key);
                        *last_user_input_change_time_for_listener.lock().unwrap() = Instant::now();
                    }
                }

                _ => {}
            }
        })
    {
        eprintln!("Error: {:?}", error);
    }
}
