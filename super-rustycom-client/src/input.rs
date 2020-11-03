use std::collections::{HashMap, HashSet};

use log::info;
use minifb::{Key, MouseButton, MouseMode, Window};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    JustPressed,
    Held,
    JustReleased,
    Up,
}

pub struct InputState {
    keyboard: KeyboardState,
    mouse: MouseState,
}

impl InputState {
    pub fn new() -> InputState {
        InputState {
            keyboard: KeyboardState::new(),
            mouse: MouseState::new(),
        }
    }

    #[allow(dead_code)]
    pub fn key_state(&self, key: Key) -> State {
        self.keyboard.key_state(key)
    }

    #[allow(dead_code)]
    pub fn mouse_button_state(&self, button: MouseButton) -> State {
        self.mouse.button_state(button)
    }

    #[allow(dead_code)]
    pub fn mouse_scroll(&self) -> (f32, f32) {
        self.mouse.scroll()
    }

    #[allow(dead_code)]
    pub fn mouse_pos(&self) -> (usize, usize) {
        self.mouse.pos()
    }

    pub fn update(&mut self, window: &Window) {
        self.keyboard.update(window);
        self.mouse.update(window);
    }
}

struct KeyboardState {
    keys: HashMap<Key, State>,
}

impl KeyboardState {
    pub fn new() -> KeyboardState {
        KeyboardState {
            keys: HashMap::new(),
        }
    }

    pub fn key_state(&self, key: Key) -> State {
        if let Some(&state) = self.keys.get(&key) {
            state
        } else {
            State::Up
        }
    }

    pub fn update(&mut self, window: &Window) {
        if let Some(keys) = window.get_keys() {
            // Get state needed for updates before self.keys is mutated
            let new_pressed: HashSet<Key> = keys.into_iter().collect();
            let previous_pressed: HashSet<Key> = self
                .keys
                .iter()
                .filter(|(_, &state)| state == State::JustPressed || state == State::Held)
                .map(|(&key, _)| key)
                .collect();
            let previous_released: HashSet<Key> = self
                .keys
                .iter()
                .filter(|(_, &state)| state == State::JustReleased)
                .map(|(&key, _)| key)
                .collect();
            let released = previous_pressed.difference(&new_pressed);

            // Handle transitions for pressed modes
            for &key in &new_pressed {
                if let Some(state) = self.keys.get_mut(&key) {
                    match state {
                        State::Up | State::JustReleased => *state = State::JustPressed,
                        State::JustPressed => *state = State::Held,
                        _ => (),
                    }
                } else {
                    self.keys.insert(key, State::JustPressed);
                }
            }

            // Handle transitions for released modes
            for key in released {
                let state = self.keys.get_mut(&key).unwrap();
                match state {
                    State::JustPressed | State::Held => *state = State::JustReleased,
                    _ => (),
                }
            }
            for key in previous_released {
                if !new_pressed.contains(&key) {
                    let state = self.keys.get_mut(&key).unwrap();
                    *state = State::Up;
                }
            }
            info!("keys {:?}", self.keys);
        }
    }
}

struct MouseState {
    buttons: (State, State, State),
    scroll: (f32, f32),
    pos: (usize, usize),
}

impl MouseState {
    pub fn new() -> MouseState {
        MouseState {
            buttons: (State::Up, State::Up, State::Up),
            scroll: (0f32, 0f32),
            pos: (0, 0),
        }
    }

    pub fn button_state(&self, button: MouseButton) -> State {
        match button {
            MouseButton::Left => self.buttons.0,
            MouseButton::Middle => self.buttons.1,
            MouseButton::Right => self.buttons.2,
        }
    }

    pub fn scroll(&self) -> (f32, f32) {
        self.scroll
    }

    pub fn pos(&self) -> (usize, usize) {
        self.pos
    }

    pub fn update(&mut self, window: &Window) {
        if let Some(state) = window.get_scroll_wheel() {
            info!("Scroll state {:?}", state);
            self.scroll = state;
        } else {
            self.scroll = (0f32, 0f32);
        }
        if let Some(pos) = window.get_mouse_pos(MouseMode::Clamp) {
            info!("Mouse pos {:?}", pos);
            self.pos = (pos.0 as usize, pos.1 as usize);
        }

        macro_rules! button_handler {
            ($button:expr, $state:expr) => {
                if window.get_mouse_down($button) {
                    info!("{} down", stringify!($button));
                    match $state {
                        State::Up | State::JustReleased => $state = State::JustPressed,
                        State::JustPressed => $state = State::Held,
                        _ => (),
                    }
                } else {
                    match $state {
                        State::JustPressed | State::Held => $state = State::JustReleased,
                        State::JustReleased => $state = State::Up,
                        _ => (),
                    }
                }
            };
        };

        button_handler!(MouseButton::Left, self.buttons.0);
        button_handler!(MouseButton::Middle, self.buttons.1);
        button_handler!(MouseButton::Right, self.buttons.2);
    }
}
