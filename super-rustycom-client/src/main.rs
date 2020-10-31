mod config;
mod debugger;
mod draw_data;
mod framebuffer;
mod text;
mod time_source;

use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;

use crate::config::Config;
use crate::debugger::{disassemble_current, DebugState, Debugger};
use crate::draw_data::DrawData;
use crate::framebuffer::Framebuffer;
use crate::text::TextRenderer;
use crate::time_source::TimeSource;
use clap::clap_app;
use minifb::{Key, Window, WindowOptions};
use super_rustycom_core::snes::SNES;

const SHOWN_HISTORY_LINES: usize = 20;
// Cpu cycles to gather disassembly for
// Might be overkill without long interrupts but is still fast
const HISTORY_CYCLE_COUNT: usize = 1000;

fn main() {
    let args = clap_app!(super_rustycom_client =>
        (version: "0.1.0")
        (author: "sndels <sndels@iki.fi>")
        (about: "Very WIP Super Nintendo emulation")
        (@arg ROM: --rom +takes_value "Sets the rom file to use, previous file used if not given")
    )
    .get_matches();

    let mut config = Config::load();

    // Get ROM path from first argument
    if let Some(rom_path) = args.value_of("ROM") {
        config.rom_path = rom_path.to_string();
    }
    assert!(
        !config.rom_path.is_empty(),
        "No ROM given in args or in config"
    );

    // Load ROM from file
    let mut rom_file = File::open(&config.rom_path).expect("Opening rom failed");
    let mut rom_bytes = Vec::new();
    let read_bytes = rom_file
        .read_to_end(&mut rom_bytes)
        .expect("Reading rom to bytes failed");
    println!("Read {} bytes from {}", read_bytes, config.rom_path);

    // Init hardware
    let mut snes = SNES::new(rom_bytes);
    let mut debugger = Debugger::new();

    // Init drawing
    let mut fb = Framebuffer::new(&config);
    let mut window = Window::new(
        "Super Rustycom",
        config.resolution.width,
        config.resolution.height,
        {
            let mut options = WindowOptions::default();
            options.scale = minifb::Scale::X2;
            options.scale_mode = minifb::ScaleMode::AspectRatioStretch;
            options.resize = true;
            options
        },
    )
    .unwrap();
    //window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));
    let text_renderer = TextRenderer::new();

    let mut debug_data = DrawData::new();

    // Init time source
    let time_source = TimeSource::new();
    let mut emulated_clock_ticks = 0;

    // Run
    while window.is_open() && !window.is_key_down(Key::Escape) {
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
                snes.run_steps(debugger.steps, |cpu, mut abus| {
                    new_disassembly.push(disassemble_current(&cpu, &mut abus))
                });
                // Reset debugger state
                debugger.steps = 0;
                debugger.state = DebugState::Active;
                // Update cycle count to prevent warping on pauses
                emulated_clock_ticks = time_source.elapsed_ticks();
            }
            DebugState::Run => {
                let t_run = Instant::now();
                let (ticks, hit_breakpoint) = snes.run(
                    diff_ticks,
                    debugger.breakpoint,
                    |cpu, mut abus, ops_left| {
                        if ops_left < HISTORY_CYCLE_COUNT {
                            new_disassembly.push(disassemble_current(&cpu, &mut abus))
                        }
                    },
                );

                if hit_breakpoint {
                    debugger.state = DebugState::Active;
                }

                let emulated_nanos = TimeSource::to_nanos(ticks);
                let spent_nanos = t_run.elapsed().as_nanos();
                debug_data.extra_nanos = emulated_nanos.saturating_sub(spent_nanos);
                debug_data.missing_nanos = spent_nanos.saturating_sub(emulated_nanos);

                // Update actual number of emulated cycles
                emulated_clock_ticks += ticks;
            }
            DebugState::Quit => break,
        }

        let (w_window, h_window) = window.get_size();
        // We use double scale for output
        let w_buffer = w_window / 2;
        let h_buffer = h_window / 2;
        if w_buffer != config.resolution.width || h_buffer != config.resolution.height {
            fb.resize(w_buffer, h_buffer);
            config.resolution.width = w_buffer;
            config.resolution.height = h_buffer;
        }

        fb.clear(0x00000000);

        // Collect op history view
        debug_data.update_history(new_disassembly, SHOWN_HISTORY_LINES);
        let current_disassembly = [[
            String::from("> "),
            disassemble_current(&snes.cpu, &mut snes.abus),
        ]
        .join("")];
        let disassembly_iter = debug_data
            .disassembled_history
            .iter()
            .chain(&current_disassembly);
        let t_debug_draw = Instant::now();
        // Draw views
        text_renderer.draw(
            disassembly_iter,
            0xFFFFFFFF,
            fb.window(
                2,
                2,
                config.resolution.width - 2 - 1,
                config.resolution.height - 2 - 1,
            ),
        );
        text_renderer.draw(
            &debugger::status_str(&snes.cpu),
            0xFFFFFFFF,
            fb.window(config.resolution.width - 79, 2, 79, 85),
        );
        let debug_draw_millis = t_debug_draw.elapsed().as_nanos() as f32 * 1e-6;

        text_renderer.draw(
            &[format!["Debug draw took {:.2}ms!", debug_draw_millis]],
            0xFFFFFFFF,
            fb.window(
                2,
                config.resolution.height - 32,
                config.resolution.width,
                config.resolution.height,
            ),
        );
        if debug_data.extra_nanos > 0 {
            text_renderer.draw(
                &[format![
                    "Emulation is {:.2}ms ahead!",
                    debug_data.extra_nanos as f32 * 1e-6
                ]],
                0xFFFFFFFF,
                fb.window(
                    2,
                    config.resolution.height - 14,
                    config.resolution.width,
                    config.resolution.height,
                ),
            );
        } else if debug_data.missing_nanos > 0 {
            text_renderer.draw(
                &[format![
                    "Lagged {:2}ms behind!",
                    debug_data.missing_nanos as f32 * 1e-6
                ]],
                0xFFFF0000,
                fb.window(
                    2,
                    config.resolution.height - 14,
                    config.resolution.width,
                    config.resolution.height,
                ),
            );
        }

        if let Err(msg) = window.update_with_buffer(
            fb.buffer(),
            config.resolution.width,
            config.resolution.height,
        ) {
            eprintln!("Window: {}", msg);
        }
    }

    config.save();
}
