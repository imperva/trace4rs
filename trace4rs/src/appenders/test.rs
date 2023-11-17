use std::{fs, io::Write, path::Component, sync::Arc};

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use parking_lot::Mutex;

use super::rolling::{self, Roller, Rolling, Trigger};
use crate::{appenders::rolling::FixedWindow, Appender};

fn get_appender(path: &Utf8Path, pattern: &Option<String>) -> Appender {
    Appender::new_rolling(path.as_str(), pattern.as_deref(), 2, "10 B").unwrap()
}

fn window_roll(path: &Utf8Path, pattern: String, mut appender: Appender) {
    let buf1 = "123456789".to_string();
    appender.write_all(buf1.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    let content = fs::read_to_string(&path).unwrap();
    // not rolled, contains buf
    assert_eq!(content, buf1);

    let buf2 = "AB".to_string();
    appender.write_all(buf2.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    let content = fs::read_to_string(&path).unwrap();
    let mut buf1a2 = buf1;
    buf1a2.push_str(&buf2);
    // rolled
    assert_eq!(content, "");

    let content0_path = path
        .parent()
        .unwrap()
        .join(&pattern)
        .as_str()
        .replace(FixedWindow::INDEX_TOKEN, &0.to_string());
    println!("content0 path {}", content0_path);
    let content0 = fs::read_to_string(content0_path).unwrap();

    assert_eq!(content0, buf1a2);

    let buf3 = "CD".to_string();
    appender.write_all(buf3.as_bytes()).unwrap();
    appender.flush_io().unwrap();

    // contains buf3
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, buf3);

    // test second rolled
    let buf4 = "EF123456789".to_string();
    appender.write_all(buf4.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "");

    let mut buf3a4 = buf3.clone();
    buf3a4.push_str(&buf4);
    let content0 = fs::read_to_string(
        path.parent()
            .unwrap()
            .join(&pattern)
            .as_str()
            .replace(FixedWindow::INDEX_TOKEN, &0.to_string()),
    )
    .unwrap();
    assert_eq!(content0, buf3a4);

    let content1 = fs::read_to_string(
        path.parent()
            .unwrap()
            .join(&pattern)
            .as_str()
            .replace(FixedWindow::INDEX_TOKEN, &1.to_string()),
    )
    .unwrap();
    assert_eq!(content1, buf1a2);

    // verify we dont roll overflow
    appender.write_all(buf4.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    fs::read_to_string(
        path.parent()
            .unwrap()
            .join(&pattern)
            .as_str()
            .replace(FixedWindow::INDEX_TOKEN, &2.to_string()),
    )
    .expect_err("expected there to never be a third rolled file");
}

// Create a relative path from an absolute path
// Depending on how deep in the file system the working directory is, creating a
// relative path can cause file path length issues, especially on Windows.
// Therefore these issues are going to manifest in the dirty/non-canonical
// relative paths first.
fn as_rel_path(abs_path: &Utf8Path) -> Utf8PathBuf {
    // The path should be absolute
    assert!(abs_path.is_absolute());

    // Get the current dir on the file system
    let current_dir = std::env::current_dir().unwrap();
    let mut rel_path = Utf8PathBuf::new();

    // Create a relative path that navigates up to the root directory
    for comp in current_dir.components() {
        if let Component::Normal(_) = comp {
            rel_path = rel_path.join("..");
        }
    }

    // Add all of the components from the absolute path
    for comp in abs_path.components() {
        if let Utf8Component::Normal(n) = comp {
            rel_path = rel_path.join(n);
        }
    }

    // The path should now be relative
    assert!(!rel_path.is_absolute());

    rel_path
}

#[test]
fn correct_paths() {
    let tmpdir = tempfile::tempdir().unwrap();
    let path = tmpdir.path().join("logfile");
    let trigger = Trigger::Size { limit: 10 };
    let roller = Roller::Delete;
    let mut appender = Appender::RollingFile(Arc::new(Mutex::new(
        Rolling::new(path.to_str().unwrap(), trigger, roller).unwrap(),
    )));

    // sanity check/add some bytes to the file
    let buf1 = "123456789".to_string();
    appender.write_all(buf1.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, buf1);

    // move the file out of the way
    fs::rename(&path, tmpdir.path().join("logfile.moved")).unwrap();
    appender.correct_path().unwrap();

    // The file should be re-created
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "");

    // add some more bytes to the appender
    let buf2 = "ABCDEF".to_string();
    appender.write_all(buf2.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    let content = fs::read_to_string(&path).unwrap();
    // we expect them to be available at the correct path and to not contain the
    // prev buf
    assert_eq!(content, buf2);
}

#[test]
fn size_delete_roll() {
    let tmpdir = tempfile::tempdir().unwrap();
    let path = tmpdir.path().join("logfile");
    let trigger = Trigger::Size { limit: 10 };
    let roller = Roller::Delete;

    let mut appender = Appender::RollingFile(Arc::new(Mutex::new(
        Rolling::new(path.to_str().unwrap(), trigger, roller).unwrap(),
    )));
    let buf1 = "123456789".to_string();
    appender.write_all(buf1.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, buf1);

    let buf2 = "AB".to_string();
    appender.write_all(buf2.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "");

    let buf3 = "CD".to_string();
    appender.write_all(buf3.as_bytes()).unwrap();
    appender.flush_io().unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, buf3);
}

#[test]
fn size_window_roll() {
    let tmpdir = tempfile::tempdir().unwrap();
    let path = Utf8Path::from_path(tmpdir.path()).unwrap().join("foo.log");
    let pattern = "foo.log.{}".to_string();
    let appender = get_appender(&path, &Some(pattern.clone()));
    window_roll(&path, pattern, appender);
}

#[test]
fn size_window_roll_no_pattern() {
    let tmpdir = tempfile::tempdir().unwrap();
    let path = Utf8Path::from_path(tmpdir.path()).unwrap().join("foo.log");
    let pattern = rolling::Rolling::make_qualified_pattern(&path, None);
    let appender = get_appender(&path, &None);
    window_roll(&path, pattern, appender);
}

#[test]
fn size_window_roll_relative() {
    let tmpdir = tempfile::tempdir().unwrap();
    let path = Utf8Path::from_path(tmpdir.path()).unwrap().join("foo.log");
    let rel_path = as_rel_path(&path);
    let pattern = "foo.log.{}".to_string();
    let appender = get_appender(&rel_path, &Some(pattern.clone()));
    window_roll(&path, pattern, appender);
}

#[test]
fn size_window_roll_no_pattern_relative() {
    let tmpdir = tempfile::tempdir().unwrap();
    let path = Utf8Path::from_path(tmpdir.path()).unwrap().join("foo.log");
    let rel_path = as_rel_path(&path);
    let pattern = rolling::Rolling::make_qualified_pattern(&path, None);
    let appender = get_appender(&rel_path, &None);
    window_roll(&path, pattern, appender);
}
