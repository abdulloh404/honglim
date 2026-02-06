use rand::Rng;
use rdev::{ listen, Event, EventType, Key };
use std::{
    collections::HashSet,
    process::Command,
    sync::{ atomic::{ AtomicBool, AtomicU32, AtomicU64, Ordering }, Arc, Mutex },
    thread,
    time::{ Duration, Instant },
};

static IS_SYSTEM_INJECTING_INPUT: AtomicBool = AtomicBool::new(false);

fn run_xdotool_command(arguments: &[&str]) {
    IS_SYSTEM_INJECTING_INPUT.store(true, Ordering::Relaxed);
    let _ = Command::new("xdotool").args(arguments).status();
    IS_SYSTEM_INJECTING_INPUT.store(false, Ordering::Relaxed);
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

fn elapsed_milliseconds_since(program_start_time: Instant) -> u64 {
    program_start_time.elapsed().as_millis() as u64
}

fn is_player_manual_override_key(key: Key) -> bool {
    matches!(
        key,
        Key::Tab |
            Key::KeyQ |
            Key::KeyW |
            Key::KeyE |
            Key::KeyA |
            Key::KeyS |
            Key::KeyD |
            Key::KeyF |
            Key::ShiftLeft |
            Key::ShiftRight |
            Key::KeyZ |
            Key::KeyX |
            Key::KeyC |
            Key::Space |
            Key::KeyV |
            Key::UpArrow |
            Key::DownArrow |
            Key::LeftArrow |
            Key::RightArrow
    )
}

#[derive(Clone)]
struct ControllerContext {
    is_worker_running_flag: Arc<AtomicBool>,
    active_worker_run_id: Arc<AtomicU64>,
    this_worker_run_id: u64,

    system_pressed_keyboard_keys: Arc<Mutex<HashSet<String>>>,
    system_pressed_mouse_buttons: Arc<Mutex<HashSet<u8>>>,

    player_manual_override_key_hold_count: Arc<AtomicU32>,
    last_player_manual_override_input_at_ms: Arc<AtomicU64>,

    should_skip_next_passive_skill_after_resume: Arc<AtomicBool>,

    program_start_time: Instant,
}

impl ControllerContext {
    fn is_stop_requested(&self) -> bool {
        !self.is_worker_running_flag.load(Ordering::Relaxed) ||
            self.active_worker_run_id.load(Ordering::Relaxed) != self.this_worker_run_id
    }

    fn is_player_currently_holding_any_manual_override_key(&self) -> bool {
        self.player_manual_override_key_hold_count.load(Ordering::Relaxed) > 0
    }

    fn set_skip_next_passive_skill_after_resume_flag(&self) {
        self.should_skip_next_passive_skill_after_resume.store(true, Ordering::Relaxed);
    }

    fn wait_until_player_is_idle(&self) -> bool {
        loop {
            if self.is_stop_requested() {
                self.release_all_system_inputs();
                return false;
            }

            if self.is_player_currently_holding_any_manual_override_key() {
                self.release_all_system_inputs();
                thread::sleep(Duration::from_millis(5));
                continue;
            }

            let last_manual_ms = self.last_player_manual_override_input_at_ms.load(
                Ordering::Relaxed
            );

            if last_manual_ms == 0 {
                return true;
            }

            let now_ms = elapsed_milliseconds_since(self.program_start_time);
            if now_ms.saturating_sub(last_manual_ms) >= 500 {
                return true;
            }

            self.release_all_system_inputs();
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn ensure_player_is_idle_or_pause_worker_until_idle(&self) -> bool {
        if self.is_stop_requested() {
            self.release_all_system_inputs();
            return false;
        }

        if self.is_player_currently_holding_any_manual_override_key() {
            self.set_skip_next_passive_skill_after_resume_flag();
            self.release_all_system_inputs();
            return self.wait_until_player_is_idle();
        }

        let last_manual_ms = self.last_player_manual_override_input_at_ms.load(Ordering::Relaxed);

        if last_manual_ms != 0 {
            let now_ms = elapsed_milliseconds_since(self.program_start_time);
            if now_ms.saturating_sub(last_manual_ms) < 100 {
                self.set_skip_next_passive_skill_after_resume_flag();
                self.release_all_system_inputs();
                return self.wait_until_player_is_idle();
            }
        }

        true
    }

    fn sleep_seconds_interruptible_and_player_aware(&self, seconds: f64) -> bool {
        if seconds <= 0.0 {
            return !self.is_stop_requested();
        }

        let total_sleep_duration = Duration::from_secs_f64(seconds);
        let sleep_tick = Duration::from_millis(5);

        let mut remaining_sleep_duration = total_sleep_duration;
        let mut last_loop_time = Instant::now();

        while remaining_sleep_duration > Duration::ZERO {
            if self.is_stop_requested() {
                self.release_all_system_inputs();
                return false;
            }

            if !self.ensure_player_is_idle_or_pause_worker_until_idle() {
                return false;
            }

            let now = Instant::now();
            let elapsed_since_last_loop = now.saturating_duration_since(last_loop_time);
            last_loop_time = now;

            remaining_sleep_duration =
                remaining_sleep_duration.saturating_sub(elapsed_since_last_loop);

            let this_tick_sleep = std::cmp::min(sleep_tick, remaining_sleep_duration);
            thread::sleep(this_tick_sleep);
        }

        true
    }

    fn sleep_random_range_seconds_interruptible(&self, min_seconds: f64, max_seconds: f64) -> bool {
        let mut rng = rand::thread_rng();
        let chosen_seconds = rng.gen_range(min_seconds..max_seconds);
        self.sleep_seconds_interruptible_and_player_aware(chosen_seconds)
    }

    fn system_key_down(&self, key: &str) {
        if !self.ensure_player_is_idle_or_pause_worker_until_idle() {
            return;
        }
        {
            let mut pressed = self.system_pressed_keyboard_keys.lock().unwrap();
            pressed.insert(key.to_string());
        }
        run_xdotool_command(&["keydown", key]);
    }

    fn system_key_up(&self, key: &str) {
        if !self.ensure_player_is_idle_or_pause_worker_until_idle() {
            return;
        }
        {
            let mut pressed = self.system_pressed_keyboard_keys.lock().unwrap();
            pressed.remove(key);
        }
        run_xdotool_command(&["keyup", key]);
    }

    fn system_mouse_button_down(&self, button: u8) {
        if !self.ensure_player_is_idle_or_pause_worker_until_idle() {
            return;
        }
        {
            let mut pressed = self.system_pressed_mouse_buttons.lock().unwrap();
            pressed.insert(button);
        }
        run_xdotool_command(&["mousedown", &button.to_string()]);
    }

    fn system_mouse_button_up(&self, button: u8) {
        if !self.ensure_player_is_idle_or_pause_worker_until_idle() {
            return;
        }
        {
            let mut pressed = self.system_pressed_mouse_buttons.lock().unwrap();
            pressed.remove(&button);
        }
        run_xdotool_command(&["mouseup", &button.to_string()]);
    }

    fn system_click_mouse_button(&self, button: u8) {
        if !self.ensure_player_is_idle_or_pause_worker_until_idle() {
            return;
        }
        run_xdotool_command(&["click", &button.to_string()]);
    }

    fn system_tap_key(&self, key: &str) -> bool {
        if self.is_stop_requested() {
            self.release_all_system_inputs();
            return false;
        }
        if !self.ensure_player_is_idle_or_pause_worker_until_idle() {
            return false;
        }

        self.system_key_down(key);
        if !self.sleep_seconds_interruptible_and_player_aware(0.01) {
            return false;
        }
        self.system_key_up(key);
        true
    }

    fn should_skip_next_passive_skill_and_clear_flag(&self) -> bool {
        self.should_skip_next_passive_skill_after_resume.swap(false, Ordering::Relaxed)
    }

    fn release_all_system_inputs(&self) {
        let pressed_keys_snapshot: Vec<String> = {
            let mut pressed = self.system_pressed_keyboard_keys.lock().unwrap();
            let snapshot = pressed.iter().cloned().collect::<Vec<_>>();
            pressed.clear();
            snapshot
        };
        for key in pressed_keys_snapshot {
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

#[derive(Debug)]
struct BuffCooldownTracker {
    last_pressed_q: Option<Instant>,
    last_pressed_2: Option<Instant>,
    last_pressed_3: Option<Instant>,
    last_pressed_4: Option<Instant>,
}

impl BuffCooldownTracker {
    fn new() -> Self {
        Self {
            last_pressed_q: None,
            last_pressed_2: None,
            last_pressed_3: None,
            last_pressed_4: None,
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

        if Self::is_ready(*last_pressed, cooldown) {
            if !controller.system_tap_key(key) {
                return false;
            }
            *last_pressed = Some(Instant::now());

            if !controller.sleep_seconds_interruptible_and_player_aware(after_sleep_seconds) {
                return false;
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
                Duration::from_secs(60),
                0.75
            )
        {
            return false;
        }
        if
            !Self::press_key_if_ready(
                controller,
                "2",
                &mut self.last_pressed_2,
                Duration::from_secs(60),
                0.95
            )
        {
            return false;
        }
        if
            !Self::press_key_if_ready(
                controller,
                "3",
                &mut self.last_pressed_3,
                Duration::from_secs(60),
                0.75
            )
        {
            return false;
        }
        if
            !Self::press_key_if_ready(
                controller,
                "4",
                &mut self.last_pressed_4,
                Duration::from_secs(180),
                0.95
            )
        {
            return false;
        }

        if controller.is_stop_requested() {
            controller.release_all_system_inputs();
            return false;
        }

        if !controller.system_tap_key("z") {
            return false;
        }

        controller.release_all_system_inputs();
        true
    }
}

fn perform_passive_skill(controller: &ControllerContext) -> bool {
    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    if controller.should_skip_next_passive_skill_and_clear_flag() {
        return true;
    }

    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }

    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(0.45, 0.5) {
        return false;
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

    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.05, 0.1) {
        return false;
    }
    controller.system_click_mouse_button(1);
    controller.system_key_up("s");

    if !controller.sleep_random_range_seconds_interruptible(0.05, 0.06) {
        return false;
    }
    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.5, 0.55) {
        return false;
    }
    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.1, 0.2) {
        return false;
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

    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.05, 0.1) {
        return false;
    }

    controller.system_key_down("f");
    controller.system_key_up("f");
    controller.system_key_up("s");

    if !controller.sleep_random_range_seconds_interruptible(0.6, 0.65) {
        return false;
    }
    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(1.1, 1.15) {
        return false;
    }
    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.75, 0.8) {
        return false;
    }
    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.75, 0.8) {
        return false;
    }
    controller.system_click_mouse_button(3);

    if !controller.sleep_random_range_seconds_interruptible(0.85, 0.9) {
        return false;
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

    controller.system_key_down("s");
    controller.system_key_down("c");
    if !controller.sleep_random_range_seconds_interruptible(0.05, 0.1) {
        return false;
    }
    controller.system_key_up("s");
    controller.system_key_up("c");

    if !controller.sleep_random_range_seconds_interruptible(1.05, 1.1) {
        return false;
    }

    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    controller.system_mouse_button_down(1);
    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.05) {
        return false;
    }
    controller.system_mouse_button_up(1);
    controller.system_mouse_button_up(3);

    if !controller.sleep_random_range_seconds_interruptible(0.65, 0.7) {
        return false;
    }

    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    controller.system_key_down("q");
    controller.system_key_up("s");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    controller.system_key_up("q");

    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
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

    controller.system_key_down("Shift_L");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    controller.system_mouse_button_down(1);
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(2.2, 2.3) {
        return false;
    }

    controller.system_key_up("Shift_L");
    controller.system_mouse_button_up(1);
    controller.system_mouse_button_up(3);

    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    controller.system_key_down("f");
    if !controller.sleep_random_range_seconds_interruptible(2.0, 2.05) {
        return false;
    }
    controller.system_key_up("f");

    if !perform_passive_skill(controller) {
        return false;
    }

    controller.release_all_system_inputs();
    true
}

fn perform_strong_skill_2(controller: &ControllerContext) -> bool {
    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    controller.system_key_down("Shift_L");
    if !controller.sleep_random_range_seconds_interruptible(0.01, 0.02) {
        return false;
    }
    controller.system_key_down("f");
    if !controller.sleep_random_range_seconds_interruptible(1.1, 1.15) {
        return false;
    }
    controller.system_key_up("Shift_L");
    controller.system_key_up("f");

    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    controller.system_key_down("Shift_L");
    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(1.65, 1.7) {
        return false;
    }
    controller.system_key_up("Shift_L");
    controller.system_mouse_button_up(3);

    if !perform_passive_skill(controller) {
        return false;
    }

    if controller.is_stop_requested() {
        controller.release_all_system_inputs();
        return false;
    }

    controller.system_key_down("s");
    if !controller.sleep_random_range_seconds_interruptible(0.045, 0.05) {
        return false;
    }
    controller.system_mouse_button_down(1);
    controller.system_mouse_button_down(3);
    if !controller.sleep_random_range_seconds_interruptible(1.2, 1.25) {
        return false;
    }
    controller.system_mouse_button_up(1);
    controller.system_mouse_button_up(3);
    if !controller.sleep_random_range_seconds_interruptible(0.045, 0.05) {
        return false;
    }
    controller.system_key_up("s");

    if !controller.sleep_random_range_seconds_interruptible(0.5, 0.6) {
        return false;
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
        if !controller.sleep_random_range_seconds_interruptible(0.25, 0.3) {
            break;
        }

        if !perform_combo_1_once(&controller) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.25, 0.3) {
            break;
        }

        if !perform_combo_2_once(&controller) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.25, 0.3) {
            break;
        }

        if !perform_combo_3_once(&controller, true) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.6, 0.65) {
            break;
        }

        if !perform_combo_1_once(&controller) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.25, 0.3) {
            break;
        }

        if !perform_combo_2_once(&controller) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.25, 0.3) {
            break;
        }

        if !perform_combo_3_once(&controller, false) {
            break;
        }
        if !controller.sleep_random_range_seconds_interruptible(0.25, 0.3) {
            break;
        }

        if !buff_cooldown_tracker.apply_buffs_once(&controller) {
            break;
        }

        controller.release_all_system_inputs();
        if !controller.sleep_random_range_seconds_interruptible(0.25, 0.3) {
            break;
        }

        completed_rounds += 1;
        log_with_elapsed_time(worker_log_start_time, &format!("Round #{completed_rounds} done"));
    }

    controller.release_all_system_inputs();
    log_with_elapsed_time(worker_log_start_time, "Stopped");
}

fn main() {
    let program_start_time = Instant::now();

    let is_worker_running_flag = Arc::new(AtomicBool::new(false));
    let active_worker_run_id = Arc::new(AtomicU64::new(0));

    let system_pressed_keyboard_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
    let system_pressed_mouse_buttons = Arc::new(Mutex::new(HashSet::<u8>::new()));

    let player_manual_override_key_hold_count = Arc::new(AtomicU32::new(0));
    let last_player_manual_override_input_at_ms = Arc::new(AtomicU64::new(0));

    let should_skip_next_passive_skill_after_resume = Arc::new(AtomicBool::new(false));

    let is_worker_running_flag_for_listener = is_worker_running_flag.clone();
    let active_worker_run_id_for_listener = active_worker_run_id.clone();

    let system_pressed_keyboard_keys_for_listener = system_pressed_keyboard_keys.clone();
    let system_pressed_mouse_buttons_for_listener = system_pressed_mouse_buttons.clone();

    let player_manual_override_key_hold_count_for_listener =
        player_manual_override_key_hold_count.clone();
    let last_player_manual_override_input_at_ms_for_listener =
        last_player_manual_override_input_at_ms.clone();

    let should_skip_next_passive_skill_after_resume_for_listener =
        should_skip_next_passive_skill_after_resume.clone();

    println!("F9 = Start loop, F10 = Stop");

    if
        let Err(error) = listen(move |event: Event| {
            match event.event_type {
                EventType::KeyPress(key) => {
                    if
                        !IS_SYSTEM_INJECTING_INPUT.load(Ordering::Relaxed) &&
                        is_player_manual_override_key(key)
                    {
                        last_player_manual_override_input_at_ms_for_listener.store(
                            elapsed_milliseconds_since(program_start_time),
                            Ordering::Relaxed
                        );

                        player_manual_override_key_hold_count_for_listener.fetch_add(
                            1,
                            Ordering::Relaxed
                        );
                    }

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

                            player_manual_override_key_hold_count: player_manual_override_key_hold_count_for_listener.clone(),
                            last_player_manual_override_input_at_ms: last_player_manual_override_input_at_ms_for_listener.clone(),

                            should_skip_next_passive_skill_after_resume: should_skip_next_passive_skill_after_resume_for_listener.clone(),

                            program_start_time,
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

                            player_manual_override_key_hold_count: player_manual_override_key_hold_count_for_listener.clone(),
                            last_player_manual_override_input_at_ms: last_player_manual_override_input_at_ms_for_listener.clone(),

                            should_skip_next_passive_skill_after_resume: should_skip_next_passive_skill_after_resume_for_listener.clone(),

                            program_start_time,
                        };

                        controller.release_all_system_inputs();
                        return;
                    }
                }

                EventType::KeyRelease(key) => {
                    if IS_SYSTEM_INJECTING_INPUT.load(Ordering::Relaxed) {
                        return;
                    }

                    if is_player_manual_override_key(key) {
                        last_player_manual_override_input_at_ms_for_listener.store(
                            elapsed_milliseconds_since(program_start_time),
                            Ordering::Relaxed
                        );

                        let current_hold_count =
                            player_manual_override_key_hold_count_for_listener.load(
                                Ordering::Relaxed
                            );
                        if current_hold_count > 0 {
                            player_manual_override_key_hold_count_for_listener.store(
                                current_hold_count - 1,
                                Ordering::Relaxed
                            );
                        }
                    }
                }

                _ => {}
            }
        })
    {
        eprintln!("Error: {:?}", error);
    }
}
