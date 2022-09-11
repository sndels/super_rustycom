use log::{error, info};
use nanoserde::{DeRon, SerRon};

use std::{fs::File, io::Write, string::String};

#[derive(SerRon, DeRon)]
pub struct Resolution {
    pub width: usize,
    pub height: usize,
}

impl Resolution {
    pub fn new(width: usize, height: usize) -> Resolution {
        Resolution { width, height }
    }
}

#[derive(SerRon, DeRon)]
pub struct Config {
    pub rom_path: String,
    pub resolution: Resolution,
}

static CONFIG_PATH: &str = "config.ron";

impl Config {
    pub fn new() -> Config {
        Config {
            rom_path: String::new(),
            resolution: Resolution::new(1152, 864),
        }
    }

    pub fn load() -> Config {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(ron) => match DeRon::deserialize_ron(&ron) {
                Ok(config) => return config,
                Err(why) => {
                    error!("{}", why);
                }
            },
            Err(why) => {
                error!("{}", why);
            }
        }
        info!("Initializing new config");
        Config::new()
    }

    pub fn save(&self) {
        let ron = self.serialize_ron();
        match File::create(CONFIG_PATH) {
            Ok(mut file) => {
                if let Err(why) = file.write_all(ron.as_bytes()) {
                    error!("{}", why);
                }
            }
            Err(why) => {
                error!("{}", why);
            }
        };
    }
}
