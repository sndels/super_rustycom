mod config;
mod debugger;
mod draw_data;
mod macros;
mod time_source;
mod ui;
mod window;

use log::{error, info};
use std::{fs::File, io::prelude::*};
use super_rustycom_core::snes::Snes;

use crate::{config::Config, debugger::Debugger, window::Window};

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

const HELP: &str = "\
--rom [FILE]      Sets the rom file to use, previous file used if not given
";

struct Args {
    rom: Option<String>,
}

fn parse_args() -> Result<Args, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();
    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let args = Args {
        rom: pargs.opt_value_from_str("--rom")?,
    };

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Unused arguments: {:?}", remaining);
    }

    Ok(args)
}

fn main() {
    let args = unwrap(parse_args());

    if let Err(why) = setup_logger() {
        panic!("{}", why);
    };

    let mut config = Config::load();

    // Get ROM path from first argument
    if let Some(rom_path) = args.rom {
        config.rom_path = rom_path;
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
    let snes = Snes::new(rom_bytes);
    let debugger = Debugger::new();

    // TODO: Give mutable config, update window size for write out
    let window = Window::new("Super Rustycom", &config, snes, debugger);
    window.main_loop();

    config.save();
}
