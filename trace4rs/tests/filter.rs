#![cfg(feature = "serde")]
use std::{env, fs, thread::sleep, time::Duration};

use serde_json::json;
use trace4rs::{Config, Handle};

#[test]
fn test_filter() {
    let tmp_guard = tempfile::tempdir().unwrap();
    env::set_current_dir(tmp_guard.path()).unwrap();

    let conf: Config = serde_json::from_value(json!( {
        "root": {
            "appenders": ["file1"],
            "level" : "TRACE"
        },
        "appenders": {
            "file1": {
                "kind": "file",
                "path": "file1.log"
            },
            "file2": {
                "kind": "file",
                "path": "file2.log"
            },
            "file3": {
                "kind": "file",
                "path": "file3.log"
            }
        },
        "loggers": {
            "file2_target": {
                "appenders": ["file2"],
                "level": "TRACE"
            },
            "file2_target::with_file3_subtarget": {
                "appenders": ["file3"],
                "level": "TRACE"
            }
        }
    }))
    .unwrap();

    let _rt = tokio::runtime::Runtime::new().unwrap();
    let (_handle, s) = <Handle>::from_config(&conf).unwrap();
    tracing::subscriber::set_global_default(s).unwrap();
    tracing_log::LogTracer::init().unwrap();

    sleep(Duration::from_millis(100));
    log::info!("logging to root logger");
    log::info!(target: "file2_target", "logging to file2_target");
    log::info!(
        target: "file2_target::with_file3_subtarget",
        "logging to file2_target::with_file3_subtarget"
    );
    sleep(Duration::from_millis(100));
    let f1_content = fs::read_to_string("./file1.log").unwrap();
    assert!(f1_content.contains("root logger"));
    assert!(!f1_content.contains("file2"));
    assert!(!f1_content.contains("file3"));

    let f2_content = fs::read_to_string("./file2.log").unwrap();
    assert!(!f2_content.contains("root logger"));
    assert!(f2_content.contains("file2"));
    assert!(f2_content.contains("file3"));

    let f3_content = fs::read_to_string("./file3.log").unwrap();
    assert!(!f3_content.contains("root logger"));
    assert!(!f3_content.contains("file2_target: logging to file2_target"));
    assert!(f3_content.contains("file3"));
}
