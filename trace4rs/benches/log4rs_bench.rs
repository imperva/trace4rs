use std::env;

use criterion::{
    black_box,
    criterion_group,
    criterion_main,
    Criterion,
};
use log::LevelFilter;
use log4rs::{
    append::{
        console::ConsoleAppender,
        file::FileAppender,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller,
                trigger::size::SizeTrigger,
            },
            RollingFileAppender,
        },
    },
    config::{
        Appender,
        Logger,
        Root,
    },
    Handle,
};

fn bench_appenders(c: &mut Criterion) {
    let tmp_guard = tempfile::tempdir().unwrap();
    env::set_current_dir(tmp_guard.path()).unwrap();
    let _handle = mk_handle();

    // Create the handle
    c.bench_function("file", |b| {
        b.iter(|| {
            black_box(tracing::trace!(target: "file", "foobar"));
        })
    });
    c.bench_function("rolling_file", |b| {
        b.iter(|| {
            black_box(tracing::trace!(target: "rolling_file", "foobar"));
        })
    });
}

fn mk_handle() -> Handle {
    let file_appender = FileAppender::builder().build("file.log").unwrap();
    let rolling_file_appender = {
        let roller = FixedWindowRoller::builder()
            .build("file.log.{}", 3)
            .unwrap();
        let trigger = SizeTrigger::new(1024 * 1024);
        let roll_policy = log4rs::append::rolling_file::policy::compound::CompoundPolicy::new(
            Box::new(trigger),
            Box::new(roller),
        );
        RollingFileAppender::builder()
            .build("file.log", Box::new(roll_policy))
            .unwrap()
    };
    let config = log4rs::Config::builder()
        .appender(
            Appender::builder().build("console", Box::new(ConsoleAppender::builder().build())),
        )
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .appender(Appender::builder().build("rolling_file", Box::new(rolling_file_appender)))
        .logger(
            Logger::builder()
                .appender("console")
                .additive(false)
                .build("console", LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .appender("file")
                .additive(false)
                .build("file", LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .appender("rolling_file")
                .additive(false)
                .build("rolling_file", LevelFilter::Info),
        )
        .build(
            Root::builder()
                .appender("console")
                .build(LevelFilter::Error),
        )
        .unwrap();
    log4rs::init_config(config).unwrap()
}

criterion_group!(benches, bench_appenders);
criterion_main!(benches);
