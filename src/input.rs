use winit::{dpi::PhysicalPosition, event::MouseButton, keyboard::*};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PressState {
    previous: bool,
    current: bool,
}

impl PressState {
    pub fn new() ->  Self {
        Self {
            current: false,
            previous: false,
        }
    }

    pub fn push_back(&mut self) {
        self.previous = self.current;
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MousePosState {
    pub previous: PhysicalPosition<f64>,
    pub current: PhysicalPosition<f64>,
}

impl MousePosState {
    pub fn new() -> Self {
        Self {
            previous: PhysicalPosition::new(0., 0.),
            current: PhysicalPosition::new(0., 0.),
        }
    }

    pub fn push_back(&mut self) {
        self.previous = self.current;
    }
}

#[derive(Debug, Default, Clone)]
pub struct Input {
    pub(crate) key_states: HashMap<KeyCode, PressState>,
    pub(crate) mouse_states: HashMap<MouseButton, PressState>,
    pub(crate) mouse_pos: MousePosState,
}

impl Input {
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.key_states
            .get(&key)
            .map(|state| state.current)
            .unwrap_or_default()
    }

    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.key_states
            .get(&key)
            .map(|state| state.current && !state.previous)
            .unwrap_or_default()
    }

    pub fn key_just_released(&self, key: KeyCode) -> bool {
        self.key_states
            .get(&key)
            .map(|state| state.previous && !state.current)
            .unwrap_or_default()
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_states
            .get(&button)
            .map(|state| state.current)
            .unwrap_or_default()
    }

    pub fn mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_states
            .get(&button)
            .map(|state| state.current && !state.previous)
            .unwrap_or_default()
    }

    pub fn mouse_just_released(&self, button: MouseButton) -> bool {
        self.mouse_states
            .get(&button)
            .map(|state| state.previous && !state.current)
            .unwrap_or_default()
    }

    pub fn mouse_pos(&self) -> PhysicalPosition<f64> {
        self.mouse_pos.current
    }

    pub fn mouse_offset(&self) -> PhysicalPosition<f64> {
        PhysicalPosition {
            x: self.mouse_pos.current.x - self.mouse_pos.previous.x,
            y: self.mouse_pos.current.y - self.mouse_pos.previous.y,
        }
    }

    pub fn set_key_state(&mut self, key: KeyCode, pressed: bool) {
        self.key_states.entry(key).or_default().current = pressed;
    }

    pub fn set_mouse_state(&mut self, button: MouseButton, pressed: bool) {
        self.mouse_states.entry(button).or_default().current = pressed;
    }

    pub fn push_back(&mut self) {
        self.key_states.iter_mut().for_each(|(_, state)| state.push_back());
        self.mouse_states.iter_mut().for_each(|(_, state)| state.push_back());
        self.mouse_pos.push_back();
    }
}