use flexi_logger::{LevelFilter, LoggerHandle};
use lazy_static::lazy_static;
use std::str::FromStr;
use tracing::info;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::{prelude::*, EnvFilter};

pub fn logging() {
    better_panic::install();
    if let Err(e) = kankyo::load(false) {
        info!("couldn't load .env file: {}, this is probably fine", e);
    }
    let def_level = std::env::var("RUST_LOG").unwrap();
    #[cfg(not(feature = "tokio-console"))]
    {
        let def_level = LevelFilter::from_str(&def_level).unwrap();
        let spec = flexi_logger::LogSpecification::builder()
            .default(def_level)
            .module("sqlx", LevelFilter::Warn)
            .module("sqlx::query", LevelFilter::Warn)
            .module("sqlxmq", LevelFilter::Warn)
            .module("tiberius_search", LevelFilter::Warn)
            .build();
        flexi_logger::Logger::with(spec).start().unwrap();
    }
    #[cfg(feature = "tokio-console")]
    {
        use tracing::Level;
        let console_layer = console_subscriber::spawn();
        let filter = EnvFilter::from_default_env();
        let def_level = Level::from_str(&def_level).unwrap();
        let fmt_layer = tracing_subscriber::fmt::layer();
        tracing_subscriber::registry()
            .with(console_layer)
            .with(fmt_layer.with_filter(filter_fn(move |metadata| {
                match (metadata.module_path(), metadata.level()) {
                    (Some("sqlx"), &n) => n <= Level::WARN,
                    (Some("sqlx::query"), &n) => n <= Level::WARN,
                    (None, &n) => n <= Level::WARN,
                    (Some("_"), &n) => n <= Level::WARN,
                    (Some(""), &n) => n <= Level::WARN,
                    (Some("rocket::shield::shield"), &n) => n <= Level::WARN,
                    (Some("rocket::server"), &n) => n <= Level::WARN,
                    (Some(m), &n) => {
                        if m.starts_with("tiberius") {
                            if n <= def_level {
                                // This IF is deliberate, uncomment if you need to debug hard to filter log spam
                                //println!("Error unspec'd module: {:?}", m);
                                true
                            } else {
                                false
                            }
                        } else {
                            n <= Level::WARN
                        }
                    }
                }
            })))
            .init();
    }
}
