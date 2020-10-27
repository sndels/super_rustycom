mod config;
mod debugger;
mod framebuffer;
mod text;
mod time_source;

use std::fs::File;
use std::io::prelude::*;

use crate::config::Config;
use crate::debugger::disassemble_current;
use crate::debugger::{DebugState, Debugger};
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

    // Init time source
    let time_source = TimeSource::new();
    let mut emulated_clock_ticks = 0;

    // Init drawing
    let mut fb = Framebuffer::new(&config);
    let mut window = Window::new(
        "Super Rustycom",
        config.resolution.width,
        config.resolution.height,
        {
            let mut options = WindowOptions::default();
            options.scale = minifb::Scale::X4;
            options
        },
    )
    .unwrap();
    // window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));
    let text_renderer = TextRenderer::new();

    // Run
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Update time
        // TODO: Final timing in executed cpu cycles inside the emu struct (?), so keep unspent
        // clock cycles in mind
        let current_ns = time_source.elapsed_ns();

        // Calculate clock pulses, cpu cycles that should have passed
        let clock_ticks = current_ns / 47;
        let diff_ticks = (clock_ticks - emulated_clock_ticks) as u64;

        // Handle debugger state
        match debugger.state {
            DebugState::Active => {
                debugger.take_command(&mut snes.cpu, &mut snes.abus);
                // Update cycle count to prevent warping
                emulated_clock_ticks = current_ns / 47;
            }
            DebugState::Step => {
                // Go through steps
                snes.run_steps(debugger.steps, debugger.disassemble, disassemble_current);
                // Reset debugger state
                debugger.steps = 0;
                debugger.state = DebugState::Active;
                // Update cycle count to prevent warping
                emulated_clock_ticks = current_ns / 47;
            }
            DebugState::Run => {
                let (ticks, hit_breakpoint) = snes.run(
                    diff_ticks,
                    debugger.breakpoint,
                    debugger.disassemble,
                    disassemble_current,
                );
                if hit_breakpoint {
                    debugger.state = DebugState::Active;
                }
                // Update emulated cycles and take overshoot into account
                emulated_clock_ticks += ticks;
            }
            DebugState::Quit => break,
        }

        text_renderer.draw(debugger::status_str(&snes.cpu), fb.window(240, 2, 79, 85));

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
