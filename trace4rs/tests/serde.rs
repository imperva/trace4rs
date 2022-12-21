#![cfg(feature = "serde")]
use std::convert::TryInto;

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
            "path": "./foobar.log"
        },
        "file2": {
            "kind": "rollingfile",
            "path": "./foobar.log",
            "rolloverPolicy": {
                "maximumFileSize": "1mb",
                "maxSizeRollBackups": 3
            }
        },
        "file3": {
            "kind": "rollingfile",
            "path": "./foobar.log",
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
    let tmp_path = tmp_guard.path().to_string_lossy();

    let conf = r#"
            {
                "root": {
                    "level" : "trace",
                    "appenders": ["file1"]
                },
                "appenders": {
                    "file1": {
                        "kind": "file",
                        "path": "<tmp_path>/foobar.log"
                    },
                    "file2": {
                        "kind": "rollingfile",
                        "path": "<tmp_path>/foobar.log",
                        "rolloverPolicy": {
                            "maximumFileSize": "1mb",
                            "maxSizeRollBackups": 3
                        }
                    },
                    "file3": {
                        "kind": "rollingfile",
                        "path": "<tmp_path>/foobar.log",
                        "rolloverPolicy": {
                            "maximumFileSize": "1mb",
                            "maxSizeRollBackups": 3,
                            "pattern": "<tmp_path>/foobar.log.roll.{}"
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
    let conf = conf.replace("<tmp_path>", &tmp_path);
    let parsed: Config = serde_json::from_str(&conf).unwrap();

    assert_eq!(parsed.default.level, LevelFilter::TRACE);
    let file1 = &AppenderId("file1".to_string());
    assert!(parsed.default.appenders.contains(file1));

    let my_target = parsed.loggers.get(&Target::from("my_target")).unwrap();
    assert_eq!(my_target.level, LevelFilter::WARN);
    assert_eq!(my_target.appenders.len(), 1);
    assert_eq!(my_target.appenders.iter().next().unwrap(), file1);

    let file1_appender = parsed.appenders.get(file1).unwrap();
    assert_eq!(file1_appender, &Appender::File {
        path: format!("{tmp_path}/foobar.log"),
    });

    // now lets convert this to a Handle
    let _handle: crate::Handle = parsed.try_into().unwrap();
}

#[test]
fn test_custom_parse_fail() {
    // Lets not leave the git dir filthy.
    let tmp_guard = tempfile::tempdir().unwrap();
    let tmp_path = tmp_guard.path().to_string_lossy();

    let conf = r#"
            {
                "root": {
                    "level" : "trace",
                    "appenders": ["file1"],
                    "format" : {
                        "custom": "{f}",
                        "badfmtkey": "badfmtvalue"
                    }
                },
                "appenders": {
                    "file1": {
                        "kind": "file",
                        "path": "<tmp_path>/foobar.log"
                    }
                },
                "loggers": {
                    "my_target": {
                        "level": "warn",
                        "appenders": ["file1"],
                        "format": "messageonly"
                    }
                }
            }
        "#;
    let conf = conf.replace("<tmp_path>", &tmp_path);
    if let Err(parse_err) = serde_json::from_str::<Config>(&conf) {
        assert!(parse_err.to_string().contains("did not match any variant"));
    } else {
        panic!("expected parse to fail")
    }
}
