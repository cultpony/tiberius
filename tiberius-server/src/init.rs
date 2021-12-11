use flexi_logger::{LevelFilter, LoggerHandle};
use lazy_static::lazy_static;
use tracing::info;

lazy_static! {
    pub static ref LOGGER: LoggerHandle = {
        better_panic::install();
        if let Err(e) = kankyo::load(false) {
            info!("couldn't load .env file: {}, this is probably fine", e);
        }
        flexi_logger::Logger::with(
            flexi_logger::LogSpecification::builder()
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
