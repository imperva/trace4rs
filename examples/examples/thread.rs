use std::{
    fs,
    sync::Arc,
    thread,
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

fn main() {
    let tmp_guard = tempfile::tempdir().unwrap();
    let file_out = tmp_guard.path().join("file.log");
    let file_out_lossy = file_out.to_string_lossy();

    // Create the handle
    let config = {
        let console = config::Appender::console();
        let file = config::Appender::file(file_out_lossy.clone().into_owned());
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
        Config {
            default,
            loggers: literally::hmap! {"trace4rs" => l1},
            appenders,
        }
    };
    let (h, s) = <Handle>::from_config(&config).unwrap();

    tracing::subscriber::set_global_default(s).unwrap();
    println!("Created subscribler for {}", file_out_lossy);

    let arcd_h = Arc::new(h);
    // Spawn an thread to correct appender paths
    let file_out_lossy_clone = file_out_lossy.clone().into_owned();
    let interval = 500;
    thread::spawn(move || {
        loop {
            println!("Correcting the append for {}", file_out_lossy_clone);
            arcd_h.correct_appender_paths().unwrap();
            thread::sleep(Duration::from_millis(interval));
        }
    });

    // Alternate between removing and checking on the file
    for i in 0..10 {
        if i % 2 == 0 {
            println!("Removing file {}", file_out_lossy);
            fs::remove_file(&file_out).unwrap();
        } else {
            println!("Check on file {}", file_out_lossy);
            fs::File::open(&file_out).unwrap();
        }
        thread::sleep(Duration::from_millis(interval * 3));
    }
}
