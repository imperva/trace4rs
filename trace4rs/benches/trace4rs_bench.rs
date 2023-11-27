#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use std::env;

use criterion::{black_box, Criterion};
use trace4rs::{
    config::{self, Format, Policy},
    Config, Handle,
};
use tracing::Subscriber;
macro_rules! do_log {
    (target: $target:literal, $($rst:tt)*) => {{
        #[cfg(not(feature = "tracing-macros"))]
        black_box(log::info!(target: $target, $($rst)*));
        #[cfg(feature = "tracing-macros")]
        black_box(tracing::info!(target: $target, $($rst)*));
    }};
}

#[criterion_macro::criterion]
fn bench_appenders(c: &mut Criterion) {
    let tmp_guard = tempfile::tempdir().unwrap();
    env::set_current_dir(tmp_guard.path()).unwrap();
    let (_h, s) = mk_handle();

    // Create the handle
    tracing::subscriber::set_global_default(s).unwrap();
    c.bench_function("tracing_file", |b| {
        b.iter(|| do_log!(target: "file", "foobar"))
    });
    c.bench_function("tracing_rolling_file", |b| {
        b.iter(|| do_log!(target: "rolling_file", "foobar"))
    });
}

fn mk_handle() -> (Handle, impl Subscriber) {
    let appenders = {
        let console = config::Appender::console();
        let file = config::Appender::File {
            path: "file.log".to_string(),
        };
        let rolling_file = config::Appender::RollingFile {
            path: "rolling_file.log".to_string(),
            policy: Policy {
                pattern: Some("rolling_file.log.{}".to_string()),
                max_size_roll_backups: 3,
                maximum_file_size: "1mb".to_string(),
            },
        };

        literally::hmap! {
            "console" => console,
            "file" => file,
            "rolling_file" => rolling_file,
        }
    };

    let default = config::Logger {
        level: config::LevelFilter::INFO,
        appenders: literally::hset! {"console"},
        format: Format::default(),
    };
    let loggers = {
        let file_logger = config::Logger {
            level: config::LevelFilter::INFO,
            appenders: literally::hset! {"file"},
            format: Format::default(),
        };
        let rolling_file_logger = config::Logger {
            level: config::LevelFilter::INFO,
            appenders: literally::hset! {"file"},
            format: Format::default(),
        };
        literally::hmap! {"file" => file_logger, "rolling_file" => rolling_file_logger}
    };

    let config = Config {
        default,
        loggers,
        appenders,
    };

    Handle::from_config(&config).unwrap()
}
