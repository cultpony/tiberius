use flexi_logger::LoggerHandle;
use lazy_static::lazy_static;
use log::{info, LevelFilter};

lazy_static! {
    pub static ref LOGGER: LoggerHandle = {
        better_panic::install();
        if let Err(e) = kankyo::load(false) {
            info!("couldn't load .env file: {}, this is probably fine", e);
        }
        flexi_logger::Logger::with(
            flexi_logger::LogSpecification::default(LevelFilter::Warn)
                .module("sqlx", LevelFilter::Warn)
                .module("sqlx::query", LevelFilter::Warn)
                .module("sqlxmq", LevelFilter::Info)
                .module("tiberius", LevelFilter::Trace)
                .module("tiberius_models", LevelFilter::Trace)
                .module("tiberius_jobs", LevelFilter::Trace)
                .module("tiberius_core", LevelFilter::Trace)
                .module("philomena_searcher", LevelFilter::Trace)
                .build(),
        )
        .start()
        .unwrap()
    };
}
