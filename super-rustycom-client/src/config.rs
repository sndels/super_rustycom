use serde::{Deserialize, Serialize};
use serde_json;

use std::{
    fs::File,
    io::{BufReader, BufWriter},
    string::String,
};

#[derive(Serialize, Deserialize)]
pub struct Resolution {
    pub width: usize,
    pub height: usize,
}

impl Resolution {
    pub fn new(width: usize, height: usize) -> Resolution {
        Resolution { width, height }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub rom_path: String,
    pub resolution: Resolution,
}

static CONFIG_PATH: &str = "config.json";

impl Config {
    pub fn new() -> Config {
        Config {
            rom_path: String::new(),
            resolution: Resolution::new(320, 240),
        }
    }

    pub fn load() -> Config {
        match File::open(CONFIG_PATH) {
            Ok(config_file) => {
                let reader = BufReader::new(config_file);
                serde_json::from_reader(reader).expect("Reading existing config failed!")
            }
            Err(_) => Config::new(),
        }
    }

    pub fn save(&self) {
        let config_file = File::create(CONFIG_PATH).expect("Opening config for write failed!");
        let writer = BufWriter::new(config_file);
        serde_json::to_writer(writer, &self).expect("Writing config failed!");
    }
}
