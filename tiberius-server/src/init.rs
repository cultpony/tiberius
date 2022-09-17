use lazy_static::lazy_static;
use std::str::FromStr;
use tiberius_core::config::Configuration;
use tiberius_dependencies::sentry;
use tracing::info;
use tracing_subscriber::{filter::filter_fn, prelude::*, EnvFilter};

pub fn logging(config: &Configuration) {
    better_panic::install();
    let def_level = config.log_level.into();
    use tracing::Level;
    let filter = EnvFilter::from_default_env();
    let fmt_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(
            fmt_layer
                .with_filter(tracing::metadata::LevelFilter::from_level(def_level))
                .with_filter(filter_fn(|metadata| -> bool {
                    //println!("MODULE: metadata.module: {:?}", metadata.module_path());
                    match metadata.module_path() {
                        None => true,
                        Some(v) => !{
                            let v = v.to_ascii_lowercase();
                            v.contains("hyper")
                                || v.contains("html5ever")
                                || v.contains("runtime")
                                || v.contains("want")
                                || v.contains("reqwest")
                                || v.contains("tokio")
                                || v.contains("mio")
                                || v.contains("tantivy")
                                || v.contains("sqlx")
                        },
                    }
                })),
        )
        .with(sentry::integrations::tracing::layer())
        .init();
}
