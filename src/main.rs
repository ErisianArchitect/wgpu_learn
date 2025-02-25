#![allow(unused)]

use pollster;
use wgpu_learn::state::State;
use std::{collections::HashMap, ops::ControlFlow, time::{Duration, Instant}};
use image::{
    ImageBuffer, Rgba,
};

use winit::{
    dpi::{LogicalSize, PhysicalSize, Size}, event::*, event_loop::{EventLoop, EventLoopWindowTarget}, keyboard::{KeyCode, PhysicalKey}, window::WindowBuilder
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Timer(Instant);
impl Timer {
    fn time(&mut self) -> Duration {
        let duration = self.0.elapsed();
        self.0 = Instant::now();
        duration
    }

    fn framerate(self) -> f64 {
        1.0 / self.0.elapsed().as_secs_f64()
    }

    fn wait(&mut self, duration: Duration) {
        spin_sleep::sleep_until(self.0 + duration);
        self.0 = Instant::now();
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().with_inner_size(Size::Logical(LogicalSize::new(1280.0, 720.0))).build(&event_loop).unwrap();

    let mut state = State::new(&window).await;
    let monitor = state.window().current_monitor().unwrap();
    if let Some(refresh) = monitor.refresh_rate_millihertz() {
        println!("Refresh rate: {}", refresh / 1000);
    }
    let mut timer = Timer(Instant::now());
    let mut wait_timer = Timer(Instant::now());
    event_loop.run(move |event, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == state.window.id() => if !state.input(event) {
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        // Escape key pressed
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => control_flow.exit(),
                WindowEvent::Focused(focus) => {
                    println!("Focus: {focus}");
                }
                WindowEvent::Resized(physical_size) => {
                    state.resize(*physical_size);
                }
                WindowEvent::RedrawRequested => {
                    let old = timer;
                    // timer.wait(Duration::from_secs(1)/60);
                    let time = timer.time();
                    println!("Framerate: {}", old.framerate());
                    state.update();
                    match state.render() {
                        Ok(_) => {},
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("OutOfMemory");
                            control_flow.exit()
                        },
                        Err(wgpu::SurfaceError::Timeout) => {
                            log::warn!("Surface timeout");
                        }
                        Err(e) => eprintln!("{e:?}"),
                    }
                }
                _ => {}
            }
        }
        Event::AboutToWait => {
            let wait = wait_timer;
            wait_timer.time();
            println!("Wait FPS: {}", wait.framerate());
            state.window().request_redraw();
        }
        _ => {}
    });
}

#[pollster::main]
async fn main() {
    pollster::block_on(run());
}


#[cfg(test)]
mod testing_sandbox {
    // TODO: Remove this sandbox when it is no longer in use.
    use super::*;
    #[test]
    fn sandbox() {
        struct Solution;
        impl Solution {
    pub fn unique_paths(m: i32, n: i32) -> i32 {
        fn count_paths(m: usize, n: usize, memo: &mut [[i32; 101]; 101]) -> i32 {
            if memo[m][n] != 0 {
                memo[m][n]
            } else {
                // if m == 1 || n == 1 {
                //     memo[m][n] = 1;
                //     1
                // } else {
                // }
                let mut count = 0;
                if m > 0 {
                    count += count_paths(m - 1, n, memo);
                }
                if n > 0 {
                    count += count_paths(m, n - 1, memo);
                }
                count
            }
        }
        let mut memo = [[0; 101]; 101];
        memo[0][0] = 1;
        count_paths(m as usize - 1, n as usize - 1, &mut memo)
    }
        }
    }
}