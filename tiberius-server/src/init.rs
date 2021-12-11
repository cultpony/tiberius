use flexi_logger::{LevelFilter, LoggerHandle};
use lazy_static::lazy_static;
use tracing::info;
use std::str::FromStr;

lazy_static! {
    pub static ref LOGGER: LoggerHandle = {
        better_panic::install();
        if let Err(e) = kankyo::load(false) {
            info!("couldn't load .env file: {}, this is probably fine", e);
        }
        let def_level = std::env::var("RUST_LOG").unwrap();
        let def_level = LevelFilter::from_str(&def_level).unwrap();
        flexi_logger::Logger::with(
            flexi_logger::LogSpecification::builder()
                .default(def_level)
                .module("sqlx", LevelFilter::Warn)
                .module("sqlx::query", LevelFilter::Warn)
                .module("sqlxmq", LevelFilter::Warn)
                .module("tiberius_search", LevelFilter::Warn)
                .build(),
        )
        .start()
        .unwrap()
    };
}
