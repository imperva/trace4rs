use std::{
    convert::TryFrom,
    sync::Arc,
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
    let handle = {
        let file = config::Appender::File {
            path: "./file.log".into(),
        };
        let default = config::Logger {
            level:     config::LevelFilter::INFO,
            appenders: literally::hset! {"file"},
            format:    Format::default(),
        };
        let config = Config {
            default,
            loggers: Default::default(),
            appenders: literally::hmap! {"file" => file},
        };

        Arc::new(Handle::try_from(config).unwrap())
    };
    tracing::subscriber::set_global_default(handle.subscriber()).unwrap();

    for i in 0..usize::MAX {
        info!("log message: {}", i);
        sleep(Duration::from_millis(500));
    }
}
