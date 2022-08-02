use log::{error, info};
use serde::{Deserialize, Serialize};

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

static CONFIG_PATH: &str = "config.yaml";

impl Config {
    pub fn new() -> Config {
        Config {
            rom_path: String::new(),
            resolution: Resolution::new(800, 600),
        }
    }

    pub fn load() -> Config {
        match File::open(CONFIG_PATH) {
            Ok(config_file) => {
                let reader = BufReader::new(config_file);
                match serde_yaml::from_reader(reader) {
                    Ok(config) => return config,
                    Err(why) => {
                        error!("{}", why);
                    }
                }
            }
            Err(why) => {
                error!("{}", why);
            }
        }
        info!("Initializing new config");
        Config::new()
    }

    pub fn save(&self) {
        let config_file = match File::create(CONFIG_PATH) {
            Ok(file) => file,
            Err(why) => {
                error!("{}", why);
                return;
            }
        };
        let writer = BufWriter::new(config_file);
        if let Err(why) = serde_yaml::to_writer(writer, &self) {
            error!("{}", why);
        }
    }
}
