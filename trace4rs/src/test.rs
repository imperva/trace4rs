#![cfg(test)]

use std::{
    convert::TryFrom,
    fs,
    io::Read,
};

use trace4rs_config::config::{
    Appender,
    Config,
    Format,
    LevelFilter,
    Logger,
};
use tracing::Subscriber;

use crate::{
    Handle,
    TraceLogger,
};

static_assertions::assert_impl_all!(TraceLogger: Subscriber, Send, Sync);

#[test]
fn test_set_global() {
    let tmp_guard = tempfile::tempdir().unwrap();
    let file_out = tmp_guard.path().join("file.log");

    let handle = {
        let console = Appender::Console;
        let file = Appender::File {
            path: file_out.to_string_lossy().into_owned(),
        };
        let appenders = literally::hmap! {
            "console" => console,
            "file" => file,
        };
        let default = Logger {
            level:     LevelFilter::WARN,
            appenders: literally::hset! {"console"},
            format:    Format::default(),
        };
        let l1 = Logger {
            level:     LevelFilter::INFO,
            appenders: literally::hset! {"file"},
            format:    Format::default(),
        };
        let config = Config {
            default,
            loggers: literally::hmap! {"trace4rs" => l1},
            appenders,
        };

        Handle::try_from(config).unwrap()
    };
    tracing::subscriber::set_global_default(handle.subscriber()).unwrap();

    each_level();
    handle.flush().unwrap();

    let mut f = fs::File::open(&file_out).unwrap();
    let mut file_content = String::new();
    f.read_to_string(&mut file_content).unwrap();

    assert!(file_content.contains("hello info"));
    assert!(file_content.contains("hello warn"));
    assert!(file_content.contains("hello error"));

    assert!(!file_content.contains("hello debug"));
    assert!(!file_content.contains("hello trace"));

    // reset the content
    file_content.clear();

    handle.disable().unwrap();
    each_level();
    f.read_to_string(&mut file_content).unwrap();

    assert!(file_content.is_empty());
}

fn each_level() {
    tracing::trace!("hello trace");
    tracing::debug!("hello debug");
    tracing::info!("hello info");
    tracing::warn!("hello warn");
    tracing::error!("hello error");
}
