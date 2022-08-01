mod config;
mod debugger;
mod draw_data;
mod framebuffer;
mod input;
mod text;
mod time_source;
mod ui;

use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;

use crate::config::Config;
use crate::debugger::{disassemble_current, DebugState, Debugger};
use crate::draw_data::DrawData;
use crate::input::{InputState, KeyState};
use crate::time_source::TimeSource;
use crate::ui::UI;
use clap::clap_app;
use log::{error, info};
use minifb::{Key, Window, WindowOptions};
use super_rustycom_core::snes::SNES;

const SHOWN_HISTORY_LINES: usize = 50;
// Cpu cycles to gather disassembly for
// Might be overkill without long interrupts but is still fast
const HISTORY_CYCLE_COUNT: usize = 1000;

fn unwrap<T, E>(result: Result<T, E>) -> T
where
    E: std::fmt::Display,
{
    match result {
        Ok(val) => val,
        Err(why) => {
            error!("{}", why);
            panic!();
        }
    }
}

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}:{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.level(),
                record.target(),
                record.line().unwrap_or(0),
                message
            ))
        })
        // .level(log::LevelFilter::Info)
        // .level(log::LevelFilter::Debug)
        .level(log::LevelFilter::Warn)
        // .level(log::LevelFilter::Error)
        .chain(std::io::stdout())
        .chain(std::fs::File::create("emu.log")?)
        .apply()?;
    Ok(())
}

fn main() {
    let args = clap_app!(super_rustycom_client =>
        (version: "0.1.0")
        (author: "sndels <sndels@iki.fi>")
        (about: "Very WIP Super Nintendo emulation")
        (@arg ROM: --rom +takes_value "Sets the rom file to use, previous file used if not given")
    )
    .get_matches();

    if let Err(why) = setup_logger() {
        panic!("{}", why);
    };

    let mut config = Config::load();

    // Get ROM path from first argument
    if let Some(rom_path) = args.value_of("ROM") {
        config.rom_path = rom_path.to_string();
    }
    if config.rom_path.is_empty() {
        error!("No ROM given in args or in config");
        panic!();
    }

    // Load ROM from file
    let rom_bytes = {
        let mut rom_file = unwrap(File::open(&config.rom_path));
        let mut rom_bytes = Vec::new();
        let read_bytes = unwrap(rom_file.read_to_end(&mut rom_bytes));
        info!("Read {} bytes from {}", read_bytes, config.rom_path);
        rom_bytes
    };

    // Init hardware
    let mut snes = SNES::new(rom_bytes);
    let mut debugger = Debugger::new();

    // Init drawing
    let mut window = unwrap(Window::new(
        "Super Rustycom",
        config.resolution.width,
        config.resolution.height,
        WindowOptions {
            scale: minifb::Scale::X2,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            resize: true,
            ..WindowOptions::default()
        },
    ));

    let mut ui = UI::new(&config);

    let mut input_state = InputState::new();

    let mut debug_data = DrawData::new();

    // Init time source
    let time_source = TimeSource::new();
    let mut emulated_clock_ticks = 0;

    // Run
    debugger.state = DebugState::Run;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Update ticks that should have passed
        let clock_ticks = time_source.elapsed_ticks();
        let diff_ticks = clock_ticks.saturating_sub(emulated_clock_ticks);

        input_state.update(&window);

        // Give debugger control on space
        if input_state.key_state(Key::Space) == KeyState::JustPressed {
            debugger.state = DebugState::Active;
        }

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
            ui.resize(w_buffer, h_buffer);
            config.resolution.width = w_buffer;
            config.resolution.height = h_buffer;
        }

        debug_data.update_history(new_disassembly, SHOWN_HISTORY_LINES);

        ui.draw(&debug_data, &mut snes, &config);

        if let Err(why) = window.update_with_buffer(
            ui.buffer(),
            config.resolution.width,
            config.resolution.height,
        ) {
            error!("{}", why);
        }
    }

    config.save();
}
