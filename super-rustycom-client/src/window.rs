use glium::glutin;
use glium::{
    glutin::{
        dpi::LogicalSize,
        event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        platform::run_return::EventLoopExtRunReturn,
        window::WindowBuilder,
    },
    Surface,
};
use std::time::Instant;
use super_rustycom_core::snes::SNES;

use crate::{
    config::Config,
    debugger::{disassemble_current, DebugState, Debugger},
    draw_data::DrawData,
    expect,
    time_source::TimeSource,
    ui::UI,
};

const SHOWN_HISTORY_LINES: usize = 50;
// Cpu cycles to gather disassembly for
// Might be overkill without long interrupts but is still fast
const HISTORY_CYCLE_COUNT: usize = 1000;

pub struct Window {
    // Window and GL context
    event_loop: EventLoop<()>,
    display: glium::Display,

    ui: UI,

    snes: SNES,
    debugger: Debugger,
}

impl Window {
    pub fn new(title: &str, config: &Config, snes: SNES, debugger: Debugger) -> Self {
        // Create window and gl context
        let event_loop = EventLoop::new();
        let window_builder = WindowBuilder::new()
            .with_title(title.to_owned())
            .with_inner_size(LogicalSize::new(
                config.resolution.width as f64,
                config.resolution.height as f64,
            ));
        let context_builder = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_double_buffer(Some(true));
        let display = expect!(
            glium::Display::new(window_builder, context_builder, &event_loop),
            "Failed to initialize glium display"
        );

        let ui = UI::new(&display);

        Window {
            event_loop,
            display,
            ui,
            snes,
            debugger,
        }
    }

    pub fn main_loop(self) {
        let Window {
            mut event_loop,
            display,
            mut ui,
            mut snes,
            mut debugger,
        } = self;

        let mut quit = false;
        let mut last_frame_instant = Instant::now();
        let mut any_item_active = false;
        let time_source = TimeSource::new();
        let mut emulated_clock_ticks = 0;
        let mut draw_data = DrawData::new();

        while !quit {
            let gl_window = display.gl_window();
            let window = gl_window.window();

            event_loop.run_return(|event, _, control_flow| {
                ui.handle_event(window, &event);
                match event {
                    Event::NewEvents(_) => {
                        let now = Instant::now();
                        ui.update_delta_time(now - last_frame_instant);
                        last_frame_instant = now;
                    }
                    Event::MainEventsCleared => {
                        // Ran out of events so let's jump back out
                        *control_flow = ControlFlow::Exit;
                    }
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CloseRequested => {
                            quit = true;
                        }
                        WindowEvent::Resized(size) => {
                            display.gl_window().resize(size);
                        }
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(key),
                                    state: ElementState::Pressed,
                                    ..
                                },
                            ..
                        } => {
                            if !any_item_active {
                                // We only want to handle keypresses if we're not interacting with imgui
                                match key {
                                    VirtualKeyCode::Space => {
                                        debugger.state = DebugState::Active;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            });

            // "Tick" update
            {
                // Update ticks that should have passed
                let clock_ticks = time_source.elapsed_ticks();
                let diff_ticks = clock_ticks.saturating_sub(emulated_clock_ticks);

                // Handle debugger state and run the emulator
                let mut new_disassembly = Vec::new();
                match debugger.state {
                    DebugState::Active => {
                        debugger.take_command(&mut snes.cpu, &mut snes.abus);
                        // Update cycle count to prevent warping
                        emulated_clock_ticks = time_source.elapsed_ticks();
                    }
                    DebugState::Step => {
                        // Go through steps
                        snes.run_steps(debugger.steps, |cpu, abus| {
                            new_disassembly.push(disassemble_current(cpu, abus))
                        });
                        // Reset debugger state
                        debugger.steps = 0;
                        debugger.state = DebugState::Active;
                        // Update cycle count to prevent warping on pauses
                        emulated_clock_ticks = time_source.elapsed_ticks();
                    }
                    DebugState::Run => {
                        let t_run = Instant::now();
                        let (ticks, hit_breakpoint) =
                            snes.run(diff_ticks, debugger.breakpoint, |cpu, abus, ops_left| {
                                if ops_left < HISTORY_CYCLE_COUNT {
                                    new_disassembly.push(disassemble_current(cpu, abus))
                                }
                            });

                        if hit_breakpoint {
                            debugger.state = DebugState::Active;
                        }

                        let emulated_nanos = TimeSource::to_nanos(ticks);
                        let spent_nanos = t_run.elapsed().as_nanos();
                        draw_data.extra_nanos = emulated_nanos.saturating_sub(spent_nanos);
                        draw_data.missing_nanos = spent_nanos.saturating_sub(emulated_nanos);

                        // Update actual number of emulated cycles
                        emulated_clock_ticks += ticks;
                    }
                    DebugState::Quit => break,
                }

                draw_data.update_history(new_disassembly, SHOWN_HISTORY_LINES);
            }

            let mut render_target = display.draw();
            render_target.clear_color_srgb(0.0, 0.0, 0.0, 0.0);

            let ui_state = ui.render(&window, &mut render_target, &draw_data, &mut snes);

            any_item_active = ui_state.is_any_item_active;

            // Finish frame
            expect!(render_target.finish(), "Frame::finish() failed");
        }
    }
}
