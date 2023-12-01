use std::{
    thread::sleep,
    time::Duration,
};

use trace4rs::{
    config::{
        self,
        Config,
        Format,
    },
    Handle,
};
use tracing::info;

fn main() {
    // Create the handle
    let config = {
        let file = config::Appender::File {
            path: "./file.log".into(),
        };
        let default = config::Logger {
            level:     config::LevelFilter::INFO,
            appenders: literally::hset! {"file"},
            format:    Format::default(),
        };
        Config {
            default,
            loggers: Default::default(),
            appenders: literally::hmap! {"file" => file},
        }
    };
    let (_, s) = <Handle>::from_config(&config).unwrap();
    tracing::subscriber::set_global_default(s).unwrap();

    for i in 0..usize::MAX {
        info!("log message: {}", i);
        sleep(Duration::from_millis(500));
    }
}
