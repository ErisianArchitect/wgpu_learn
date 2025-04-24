#![allow(unused)]

use glam::vec3;
use pollster;
use wgpu_learn::{framepace::AverageBuffer, modeling::modeler::Modeler, state::State, FrameInfo};
use std::{collections::HashMap, ops::ControlFlow, time::{Duration, Instant}};
use image::{
    ImageBuffer, Rgba,
};

use winit::{
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize, Size}, event::*, event_loop::{EventLoop, EventLoopWindowTarget}, keyboard::{KeyCode, PhysicalKey}, monitor::VideoMode, window::WindowBuilder
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Timer(Instant);
impl Timer {
    fn start() -> Self {
        Self(Instant::now())
    }

    /// Resets the timer and returns the [Duration].
    fn time(&mut self) -> Duration {
        let duration = self.0.elapsed();
        self.0 = Instant::now();
        duration
    }

    fn split_time(&mut self) -> Self {
        let old = *self;
        self.0 = Instant::now();
        old
    }

    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn framerate(&self) -> f64 {
        1.0 / self.0.elapsed().as_secs_f64()
    }

    fn wait(&mut self, duration: Duration) {
        spin_sleep::sleep_until(self.0 + duration);
        self.0 = Instant::now();
    }
}

struct GameSettings {
    present_mode: wgpu::PresentMode,
    camera_smoothing_frame_count: Option<usize>,
    framerate_frame_count: usize,
    fullscreen: bool,
    window_title: &'static str,
    window_size: Size,
}

pub async fn run() {
    // let start_time = Instant::now();
    // let mut m = Modeler::new();
    // m.texture_index(0, move |m| {
    //     for y in 0..16 {
    //         for x in 0..16 {
    //             let xf = x as f32;
    //             let yf = y as f32;
    //             m.translate(vec3(xf, 0.0, yf), move |m| {
    //                 m.push_unit_quad();
    //             });
    //         }
    //     }
    // });
    // let elapsed = start_time.elapsed();
    // println!("{}, {}", m.vertices.len(), m.indices.len());
    // println!("{:?}", &m.vertices[4..8]);
    // println!("Elapsed: {:.06}", elapsed.as_secs_f64());
    // return;
    env_logger::init();
    let mut event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let window = WindowBuilder::new()
        .with_inner_size(Size::Logical(LogicalSize::new(1280.0, 720.0)))
        .with_title("WGPU Sandbox")
        // .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        // .with_content_protected(true)
        .build(&event_loop).unwrap();
    // window.set_cursor_visible(false);
    {
        let window_size = window.outer_size();
        let screen_size = window.current_monitor().unwrap().size();
        let window_half_size = PhysicalSize::new(window_size.width / 2, window_size.height / 2);
        let screen_half_size = PhysicalSize::new(screen_size.width / 2, screen_size.height / 2);
        let center_point = PhysicalPosition::new(
            screen_half_size.width - window_half_size.width,
            screen_half_size.height - window_half_size.height,
        );
        window.set_outer_position(center_point);
    }
    // window.set_cursor_visible(false);
    let mut state = State::new(&window).await;
    let monitor = state.window().current_monitor().unwrap();
    let frame_time = if let Some(refresh) = monitor.refresh_rate_millihertz() {
        println!("Refresh rate: {}", refresh / 1000);
        Some(refresh as f64 / 1000.0)
    } else {
        None
    };
    let mut timer = Timer(Instant::now());
    let mut wait_timer = Timer(Instant::now());
    let mut frame_counter = 0u64;
    let mut focused = true;
    let mut update_timer = Timer(Instant::now());
    let mut render_timer = Timer(Instant::now());
    let mut avg_update_time: Option<f64> = None;
    let mut avg_render_time: Option<f64> = None;
    let mut fps_avgs = AverageBuffer::new(32);

    let mut frame = FrameInfo {
        index: 0,
        fps: 0.0,
        last_frame_time: Duration::from_secs(0),
        delta_time: Duration::from_secs(0),
    };
    let mut loop_timer = Timer(Instant::now());
    event_loop.run(move |event, control_flow| {
        while let Some(event) = state.gamepad.next_event() {
            state.process_gamepad_event(&event);
        }
        state.process_event(&event);
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => if !state.process_window_event(event) {
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
                    } if state.close_requested() => control_flow.exit(),
                    WindowEvent::Focused(focus) => {
                        focused = *focus;
                        state.focus_changed(focused);
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        // if focused {
                        // }
                        let frame_time = loop_timer.split_time();
                        let fps = frame_time.framerate();
                        fps_avgs.push(fps);
                        let avg_fps = fps_avgs.average();
                        frame.fps = avg_fps;

                        frame.delta_time = frame_time.elapsed();
                        state.begin_frame(&frame);
                        // timer.wait(Duration::from_secs(1)/60);
                        
                        // println!("Framerate: {}", old.framerate());
                        {
                            let start_time = Timer::start();
                            state.update(&frame);
                            let end_time = start_time.elapsed();
                            let secs = end_time.as_secs_f64();
                            if let Some(ref mut avg) = avg_update_time {
                                *avg = (*avg + secs) * 0.5;
                            } else {
                                avg_update_time.replace(secs);
                            }
                        }

                        match state.render(&frame) {
                            Ok(render_time) => {
                                let secs = render_time.as_secs_f64();
                                if let Some(ref mut avg) = avg_render_time {
                                    *avg = (*avg + secs) * 0.5;
                                } else {
                                    avg_render_time.replace(secs);
                                }
                            },
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                log::error!("OutOfMemory");
                                control_flow.exit()
                            },
                            Err(wgpu::SurfaceError::Timeout) => {
                                log::warn!("Surface timeout");
                            }
                            Err(e) => eprintln!("Err: {e:?}"),
                        }
                        

                        let time = timer.time();
                        state.end_frame(&frame);
                        frame.last_frame_time = time;
                        frame.index += 1;
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                // println!("Wait FPS: {}", wait.framerate());
                if focused {
                    state.window().request_redraw();
                }
            }
            _ => {}
        }
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