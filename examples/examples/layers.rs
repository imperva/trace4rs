use std::{
    io,
    thread::sleep,
    time::Duration,
};

use trace4rs::{
    config::{
        self,
        Config,
        Format,
    },
    handle::{
        ExtendedT4,
        LayeredT4,
    },
    Handle,
};
use tracing::{
    info,
    Level,
    Subscriber,
};
use tracing_span_tree::SpanTree;
use tracing_subscriber::{
    filter::{
        Filtered,
        Targets,
    },
    fmt::MakeWriter,
    registry::LookupSpan,
    Layer as _,
    Registry,
};

fn main() {
    let config = {
        let file = config::Appender::File {
            path: "file.log".into(),
        };
        let default = config::Logger {
            level:     config::LevelFilter::INFO,
            appenders: literally::hset! {"file"},
            format:    Format::default(),
        };
        Config {
            default,
            loggers: Default::default(),
            appenders: literally::hmap! {"file" => file},
        }
    };

    // Create the handle
    // target can be either "" or "layers"
    let (_h, s) = init_with_metrics::<Registry, _>("", io::stdout, &config).unwrap();

    tracing::subscriber::set_global_default(s).unwrap();

    {
        let _s = tracing::span!(tracing::Level::TRACE, "foo");
        for i in 0..1_000 {
            info!("log message: {}", i);
            sleep(Duration::from_millis(1));
        }
    }
}

pub type FilteredST<Reg, Wrt> = Filtered<SpanTree<Wrt>, Targets, LayeredT4<Reg>>;

/// Init a `Handle` and `Subscriber` with span metrics collection.
/// The writer argument is where the said metrics will be written.
pub fn init_with_metrics<Reg, Wrt>(
    target: impl Into<String>,
    writer: Wrt,
    config: &Config,
) -> Result<(Handle<Reg>, ExtendedT4<Reg, FilteredST<Reg, Wrt>>), trace4rs::error::Error>
where
    Wrt: for<'a> MakeWriter<'a> + 'static,
    Reg: Subscriber + for<'a> LookupSpan<'a> + Default + Send + Sync,
{
    let layer = tracing_span_tree::span_tree_with(writer);
    let filter = Targets::new().with_target(target, Level::TRACE);
    let extra = layer.with_filter(filter);

    Handle::from_config_with(config, extra)
}
