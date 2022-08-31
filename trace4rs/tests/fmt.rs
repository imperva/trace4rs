use std::{
    env,
    fs,
    thread::sleep,
    time::Duration,
};

use serde_json::json;
use trace4rs::{
    Config,
    Handle,
};

#[test]
fn test_custom_fmt() {
    let tmp_guard = tempfile::tempdir().unwrap();
    env::set_current_dir(tmp_guard.path()).unwrap();

    let conf: Config = serde_json::from_value(json!( {
        "root": {
            "format": {
                "custom": "{t} {T}: {l} {f}",
            },
            "appenders": ["file1"],
            "level" : "TRACE"
        },
        "appenders": {
            "file1": {
                "kind": "file",
                "path": "file1.log"
            },
        },
        "loggers": {}
    }))
    .unwrap();

    let _rt = tokio::runtime::Runtime::new().unwrap();
    let handle = Handle::try_from(conf).unwrap();
    tracing::subscriber::set_global_default(handle.subscriber()).unwrap();
    tracing_log::LogTracer::init().unwrap();

    sleep(Duration::from_millis(100));
    log::info!("logging to root logger");
    sleep(Duration::from_millis(100));
    let f1_content = fs::read_to_string("./file1.log").unwrap();
    println!("{f1_content}");
    assert!(f1_content.contains("root logger"));
}
