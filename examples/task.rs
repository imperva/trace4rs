use std::{
    convert::TryFrom,
    sync::Arc,
};

use tokio::{
    fs,
    time::{
        sleep,
        Duration,
    },
};
use trace4rs::{
    config::{
        self,
        Config,
        Format,
    },
    Handle,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let tmp_guard = tempfile::tempdir().unwrap();
    let file_out = tmp_guard.path().join("file.log");
    let file_out_lossy = file_out.to_string_lossy();

    // Create the handle
    let handle = {
        let console = config::Appender::Console;
        let file = config::Appender::File {
            path: file_out_lossy.clone().into_owned(),
        };
        let appenders = literally::hmap! {
            "console" => console,
            "file" => file,
        };
        let default = config::Logger {
            level:     config::LevelFilter::WARN,
            appenders: literally::hset! {"console"},
            format:    Format::default(),
        };
        let l1 = config::Logger {
            level:     config::LevelFilter::INFO,
            appenders: literally::hset! {"file"},
            format:    Format::default(),
        };
        let config = Config {
            default,
            loggers: literally::hmap! {"trace4rs" => l1},
            appenders,
        };

        Arc::new(Handle::try_from(config).unwrap())
    };
    tracing::subscriber::set_global_default(handle.subscriber()).unwrap();
    println!("Created subscriber for {}", file_out_lossy);

    // Spawn an async task to correct appender paths
    let handle_clone = handle.clone();
    let file_out_lossy_clone = file_out_lossy.clone().into_owned();
    let interval = 500;
    tokio::spawn(async move {
        loop {
            println!("Correcting the append for {}", file_out_lossy_clone);
            handle_clone.correct_appender_paths().unwrap();
            sleep(Duration::from_millis(interval)).await;
        }
    });

    // Alternate between removing and checking on the file
    for i in 0..10 {
        if i % 2 == 0 {
            println!("Removing file {}", file_out_lossy);
            fs::remove_file(&file_out).await.unwrap();
        } else {
            println!("Check on file {}", file_out_lossy);
            fs::File::open(&file_out).await.unwrap();
        }
        sleep(Duration::from_millis(interval * 3)).await;
    }
}
