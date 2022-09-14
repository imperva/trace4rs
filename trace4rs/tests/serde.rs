#![cfg(feature = "serde")]
use std::{
    convert::TryInto,
    env,
};

use trace4rs::{
    config::{
        Appender,
        AppenderId,
        LevelFilter,
        Target,
    },
    Config,
    Handle,
};

#[test]
fn test_de_ser() {
    let conf: String = r#"{
    "root": {
        "appenders": ["file1"],
        "level" : "TRACE"
    },
    "appenders": {
        "file1": {
            "kind": "file",
            "path": "foobar.log"
        },
        "file2": {
            "kind": "rollingfile",
            "path": "foobar.log",
            "rolloverPolicy": {
                "maximumFileSize": "1mb",
                "maxSizeRollBackups": 3
            }
        },
        "file3": {
            "kind": "rollingfile",
            "path": "foobar.log",
            "rolloverPolicy": {
                "maximumFileSize": "1mb",
                "maxSizeRollBackups": 3,
                "pattern": "foobar.log.roll.{}"
            }
        }
    },
    "loggers": {
        "my_target": {
            "appenders": ["file1"],
            "level": "WARN"
        }
    }
}"#
    .chars()
    .filter(|c| !c.is_whitespace())
    .collect();

    let parsed: Config = serde_json::from_str(&conf).unwrap();
    let ser_ed = serde_json::to_string(&parsed).unwrap();

    assert_eq!(conf, ser_ed)
}

#[test]
fn test_de() {
    // Lets not leave the git dir filthy.
    let tmp_guard = tempfile::tempdir().unwrap();
    env::set_current_dir(tmp_guard.path()).unwrap();

    let conf = r#"
            {
                "root": {
                    "level" : "trace",
                    "appenders": ["file1"]
                },
                "appenders": {
                    "file1": {
                        "kind": "file",
                        "path": "foobar.log"
                    },
                    "file2": {
                        "kind": "rollingfile",
                        "path": "foobar.log",
                        "rolloverPolicy": {
                            "maximumFileSize": "1mb",
                            "maxSizeRollBackups": 3
                        }
                    },
                    "file3": {
                        "kind": "rollingfile",
                        "path": "foobar.log",
                        "rolloverPolicy": {
                            "maximumFileSize": "1mb",
                            "maxSizeRollBackups": 3,
                            "pattern": "foobar.log.roll.{}"
                        }
                    }
                },
                "loggers": {
                    "my_target": {
                        "level": "warn",
                        "appenders": ["file1"]
                    }
                }
            }
        "#;
    let parsed: Config = serde_json::from_str(conf).unwrap();

    assert_eq!(parsed.default.level, LevelFilter::TRACE);
    let file1 = &AppenderId("file1".to_string());
    assert!(parsed.default.appenders.contains(file1));

    let my_target = parsed.loggers.get(&Target::from("my_target")).unwrap();
    assert_eq!(my_target.level, LevelFilter::WARN);
    assert_eq!(my_target.appenders.len(), 1);
    assert_eq!(my_target.appenders.iter().next().unwrap(), file1);

    assert_eq!(parsed.appenders.get(file1).unwrap(), &Appender::File {
        path: "foobar.log".to_string(),
    });

    // now lets convert this to a Handle
    let _handle: crate::Handle = parsed.try_into().unwrap();
}
