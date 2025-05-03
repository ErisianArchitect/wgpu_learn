use glam::*;
use winit::{dpi::PhysicalPosition, event::MouseButton, keyboard::*};
use std::collections::{HashMap, VecDeque};

use crate::{framepace::AverageBuffer, livemouse::LiveMouse, state::Settings, FrameInfo};

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

    pub fn end_frame(&mut self) {
        self.previous = self.current;
    }
}

#[derive(Debug, Clone)]
pub struct DeltaBuffer {
    buffer: VecDeque<PhysicalPosition<f64>>,
}

impl DeltaBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        assert_ne!(capacity, 0, "Capacity can not be zero.");
        let mut new_buffer = VecDeque::with_capacity(capacity);
        new_buffer.extend(
            self
                .buffer
                .drain(self.buffer.len() - capacity.min(self.buffer.len())..self.buffer.len())
        );
        self.buffer = new_buffer;
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.buffer.len() == self.buffer.capacity()
    }

    pub fn push(&mut self, delta: PhysicalPosition<f64>) {
        if self.buffer.len() == self.buffer.capacity() {
            self.buffer.pop_front();
        }
        self.buffer.push_back(delta);
    }

    pub fn average(&self) -> PhysicalPosition<f64> {
        if self.len() == 0 {
            return PhysicalPosition::new(0.0, 0.0);
        }
        let mut total = (0.0, 0.0);
        for &pos in self.buffer.iter() {
            total.0 += pos.x;
            total.1 += pos.y;
        }
        let divisor = self.len() as f64;
        PhysicalPosition::new(total.0 / divisor, total.1 / divisor)
    }

    pub fn clear(&mut self) {
        self.buffer.clear()
    }
}

#[derive(Debug, Clone)]
pub struct MousePosState {
    pub previous: PhysicalPosition<f64>,
    pub current: PhysicalPosition<f64>,
    pub delta: PhysicalPosition<f64>,
    pub delta_avg: DeltaBuffer,
    pub live_mouse: LiveMouse,
}

impl Default for MousePosState {
    fn default() -> Self {
        Self::new()
    }
}

impl MousePosState {
    pub fn new() -> Self {
        Self {
            previous: PhysicalPosition::new(0., 0.),
            current: PhysicalPosition::new(0., 0.),
            delta: PhysicalPosition::new(0., 0.),
            delta_avg: DeltaBuffer::new(6),
            live_mouse: LiveMouse::new(100.0, 100.0, 100.0, true),
        }
    }

    pub fn begin_frame(&mut self, settings: &Settings, frame: &FrameInfo) {
        // println!("Avg.");
        // Mouse Smoothing
        self.live_mouse.update(frame.delta_time);
        if settings.mouse_halting && self.delta.x == 0.0 && self.delta.y == 0.0 {
            self.delta_avg.clear();
            self.delta_avg.push(self.delta);
        } else {
            self.delta_avg.push(self.delta);
            if settings.mouse_smoothing {
                self.delta = self.delta_avg.average();
            }
        }
    }

    pub fn end_frame(&mut self) {
        self.previous = self.current;
        self.delta = PhysicalPosition::new(0., 0.);
        // self.live_mouse.target_velocity = (0., 0.);
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

    pub fn begin_frame(&mut self, settings: &Settings, frame: &FrameInfo) {
        self.mouse_pos.begin_frame(settings, frame);
    }

    pub fn end_frame(&mut self) {
        self.key_states.retain(|_, state| {
            if !state.current {
                false
            } else {
                state.end_frame();
                true
            }
        });
        self.mouse_states.retain(|_, state| {
            if !state.current {
                false
            } else {
                state.end_frame();
                true
            }
        });
        self.mouse_pos.end_frame();
    }
}

pub struct GamepadInput {
    gilrs: gilrs::Gilrs,
    left_stick: Vec2,
    right_stick: Vec2,
    dpad: Vec2,
    left_trigger: f32,
    right_trigger: f32,
}

impl GamepadInput {
    pub fn poll(&mut self) {
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                gilrs::EventType::ButtonPressed(button, code) => {

                },
                gilrs::EventType::ButtonRepeated(button, code) => {

                },
                gilrs::EventType::ButtonReleased(button, code) => {

                },
                gilrs::EventType::ButtonChanged(button, value, _) => {

                },
                gilrs::EventType::AxisChanged(axis, value, _) => {
                    match axis {
                        gilrs::Axis::LeftStickX => self.left_stick.x = value,
                        gilrs::Axis::LeftStickY => self.left_stick.y = value,
                        gilrs::Axis::LeftZ => self.left_trigger = value,
                        gilrs::Axis::RightStickX => self.right_stick.x = value,
                        gilrs::Axis::RightStickY => self.right_stick.y = value,
                        gilrs::Axis::RightZ => self.right_trigger = value,
                        gilrs::Axis::DPadX => self.dpad.x = value,
                        gilrs::Axis::DPadY => self.dpad.y = value,
                        gilrs::Axis::Unknown => {},
                    }
                },
                gilrs::EventType::Connected => {

                },
                gilrs::EventType::Disconnected => {

                },
                gilrs::EventType::Dropped => {

                },
                gilrs::EventType::ForceFeedbackEffectCompleted => {

                },
                _ => {

                },
            }
        }
    }
}