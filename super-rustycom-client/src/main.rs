mod config;
mod debugger;
mod framebuffer;
mod text;
mod time_source;

use std::collections::VecDeque;
use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;

use crate::config::Config;
use crate::debugger::{disassemble_current, DebugState, Debugger};
use crate::framebuffer::Framebuffer;
use crate::text::TextRenderer;
use crate::time_source::TimeSource;
use clap::clap_app;
use minifb::{Key, Window, WindowOptions};
use super_rustycom_core::snes::SNES;

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
            options
        },
    )
    .unwrap();
    //window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));
    let text_renderer = TextRenderer::new();

    // We need history of ops for output
    let mut disassembled_history: VecDeque<String> = VecDeque::new();

    // Init time source
    let time_source = TimeSource::new();
    let mut emulated_clock_ticks = 0;

    // Run
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Update ticks that should have passed
        let clock_ticks = time_source.elapsed_ticks();
        let diff_ticks = clock_ticks.saturating_sub(emulated_clock_ticks);

        // Handle debugger state and run the emulator
        let mut ran_ops = Vec::new();
        let mut extra_nanos = 0;
        let mut missing_nanos = 0;
        match debugger.state {
            DebugState::Active => {
                debugger.take_command(&mut snes.cpu, &mut snes.abus);
                // Update cycle count to prevent warping
                emulated_clock_ticks = time_source.elapsed_ticks();
            }
            DebugState::Step => {
                // Go through steps
                snes.run_steps(debugger.steps, |cpu, mut abus| {
                    ran_ops.push(disassemble_current(&cpu, &mut abus))
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
                    snes.run(diff_ticks, debugger.breakpoint, |cpu, mut abus| {
                        ran_ops.push(disassemble_current(&cpu, &mut abus))
                    });

                if hit_breakpoint {
                    debugger.state = DebugState::Active;
                }

                let emulated_nanos = TimeSource::to_nanos(ticks);
                let spent_nanos = t_run.elapsed().as_nanos();
                extra_nanos = emulated_nanos.saturating_sub(spent_nanos);
                missing_nanos = spent_nanos.saturating_sub(emulated_nanos);

                // Update actual number of emulated cycles
                emulated_clock_ticks += ticks;
            }
            DebugState::Quit => break,
        }

        let t_history_gather = Instant::now();
        // Collect op history view
        disassembled_history.extend(ran_ops.into_iter());
        if disassembled_history.len() > 30 {
            disassembled_history.drain(0..disassembled_history.len() - 30);
        }
        let disassembly = [
            disassembled_history
                .iter()
                .cloned()
                .collect::<Vec<String>>()
                .join("\n"),
            [
                String::from("> "),
                disassemble_current(&snes.cpu, &mut snes.abus),
            ]
            .join(""),
        ]
        .join("\n");
        let history_gather_millis = t_history_gather.elapsed().as_nanos() as f32 * 1e-6;

        let t_debug_draw = Instant::now();
        fb.clear(0x00000000);
        // Draw views
        text_renderer.draw(
            disassembly,
            0xFFFFFFFF,
            fb.window(
                2,
                2,
                config.resolution.width - 2 - 1,
                config.resolution.height - 2 - 1,
            ),
        );
        text_renderer.draw(
            debugger::status_str(&snes.cpu),
            0xFFFFFFFF,
            fb.window(config.resolution.width - 79, 2, 79, 85),
        );
        let debug_draw_millis = t_debug_draw.elapsed().as_nanos() as f32 * 1e-6;

        text_renderer.draw(
            format!["History gahter took {:.2}ms!", history_gather_millis],
            0xFFFFFFFF,
            fb.window(
                2,
                config.resolution.height - 22,
                config.resolution.width,
                config.resolution.height,
            ),
        );
        text_renderer.draw(
            format!["Debug draw took {:.2}ms!", debug_draw_millis],
            0xFFFFFFFF,
            fb.window(
                2,
                config.resolution.height - 32,
                config.resolution.width,
                config.resolution.height,
            ),
        );
        if extra_nanos > 0 {
            text_renderer.draw(
                format!["Emulation is {:.2}ms ahead!", extra_nanos as f32 * 1e-6],
                0xFFFFFFFF,
                fb.window(
                    2,
                    config.resolution.height - 14,
                    config.resolution.width,
                    config.resolution.height,
                ),
            );
        } else if missing_nanos > 0 {
            text_renderer.draw(
                format!["Lagged {:2}ms behind!", missing_nanos as f32 * 1e-6],
                0xFFFF0000,
                fb.window(
                    2,
                    config.resolution.height - 14,
                    config.resolution.width,
                    config.resolution.height,
                ),
            );
        }

        window
            .update_with_buffer(
                fb.buffer(),
                config.resolution.width,
                config.resolution.height,
            )
            .unwrap();
    }

    config.save();
}
