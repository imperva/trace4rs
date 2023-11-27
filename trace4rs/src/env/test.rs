use std::env::{set_var, var};

use super::expand_env_vars;

#[test]
fn expand_nonexistent_env_vars_tests() {
    let path = "/empty/var/$ENV{}";
    assert_eq!(expand_env_vars(path).as_ref(), path);

    let path = "/bad/var/$ENV{GOOP}";
    assert_eq!(expand_env_vars(path).as_ref(), path);
}

#[test]
fn expand_env_vars_tests() {
    set_var("HELLO_WORLD", "GOOD BYE");

    let test_cases = {
        #[cfg(not(target_os = "windows"))]
        {
            vec![
                ("$ENV{HOME}", var("HOME").unwrap()),
                ("$ENV{HELLO_WORLD}", var("HELLO_WORLD").unwrap()),
                ("$ENV{HOME}/test", format!("{}/test", var("HOME").unwrap())),
                (
                    "/test/$ENV{HOME}",
                    format!("/test/{}", var("HOME").unwrap()),
                ),
                (
                    "/test/$ENV{HOME}/test",
                    format!("/test/{}/test", var("HOME").unwrap()),
                ),
                (
                    "/test$ENV{HOME}/test",
                    format!("/test{}/test", var("HOME").unwrap()),
                ),
                (
                    "test/$ENV{HOME}/test",
                    format!("test/{}/test", var("HOME").unwrap()),
                ),
                (
                    "$ENV{SHOULD_NOT_EXIST}",
                    "$ENV{SHOULD_NOT_EXIST}".to_string(),
                ),
                (
                    "/$ENV{HOME}/test/$ENV{SHOULD_NOT_EXIST}",
                    format!("/{}/test/$ENV{{SHOULD_NOT_EXIST}}", var("HOME").unwrap()),
                ),
            ]
        }

        #[cfg(target_os = "windows")]
        {
            vec![
                ("$ENV{HOMEPATH}", var("HOMEPATH").unwrap()),
                ("$ENV{HELLO_WORLD}", var("HELLO_WORLD").unwrap()),
                (
                    "$ENV{HOMEPATH}/test",
                    format!("{}/test", var("HOMEPATH").unwrap()),
                ),
                (
                    "/test/$ENV{USERNAME}",
                    format!("/test/{}", var("USERNAME").unwrap()),
                ),
                (
                    "/test/$ENV{USERNAME}/test",
                    format!("/test/{}/test", var("USERNAME").unwrap()),
                ),
                (
                    "/test$ENV{USERNAME}/test",
                    format!("/test{}/test", var("USERNAME").unwrap()),
                ),
                (
                    "test/$ENV{USERNAME}/test",
                    format!("test/{}/test", var("USERNAME").unwrap()),
                ),
                (
                    "$ENV{HOMEPATH}/test/$ENV{USERNAME}",
                    format!(
                        "{}/test/{}",
                        var("HOMEPATH").unwrap(),
                        var("USERNAME").unwrap()
                    ),
                ),
                (
                    "$ENV{SHOULD_NOT_EXIST}",
                    "$ENV{SHOULD_NOT_EXIST}".to_string(),
                ),
                (
                    "$ENV{HOMEPATH}/test/$ENV{SHOULD_NOT_EXIST}",
                    format!("{}/test/$ENV{{SHOULD_NOT_EXIST}}", var("HOMEPATH").unwrap()),
                ),
            ]
        }
    };

    for (input, expected) in test_cases {
        let res = expand_env_vars(input);
        assert_eq!(res, expected);
    }
}
