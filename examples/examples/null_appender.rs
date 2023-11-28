use tokio::{
    fs,
    time::{
        sleep,
        Duration,
    },
};
use trace4rs::{
    config::{
        self,
        Config,
    },
    Handle,
};

#[tokio::main]
async fn main() {
    let tmp_guard = tempfile::tempdir().unwrap();
    let log_path = tmp_guard.path().join("file.log");

    // Create the handle
    let config = {
        let default = config::Logger {
            level:     config::LevelFilter::TRACE,
            appenders: literally::hset! {"file"},
            format:    config::Format::default(),
        };
        let loggers = {
            let hush = config::Logger {
                level:     config::LevelFilter::TRACE,
                appenders: literally::hset! {"hush"},
                format:    config::Format::MessageOnly,
            };
            literally::hmap! {"hush" => hush}
        };
        let appenders = {
            let hush = config::Appender::Null;
            let file = config::Appender::File {
                path: log_path.to_string_lossy().into_owned(),
            };
            literally::hmap! {
                "file" => file,
                "hush" => hush,
            }
        };
        Config {
            default,
            loggers,
            appenders,
        }
    };
    let (_h, s) = <Handle>::from_config(&config).unwrap();
    tracing::subscriber::set_global_default(s).unwrap();
    tracing_log::LogTracer::init().unwrap();

    sleep(Duration::from_millis(100)).await;
    log::trace!(target: "hush", "this should go nowhere");
    log::trace!("this should go to file");
    sleep(Duration::from_millis(100)).await;

    println!("path is: {}", log_path.to_string_lossy());
    let file_content = fs::read_to_string(log_path).await.unwrap();
    println!("file content: {}", file_content);
    assert!(file_content.contains("this should go to file"));
    assert!(!file_content.contains("this should go nowhere"));
}
