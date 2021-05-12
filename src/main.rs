use std::io::ErrorKind;

use anyhow::Result;
use log::{debug, info, trace};
use reqwest::header::HeaderMap;
use sqlx::Postgres;
use tide::log::error;

mod app;
mod assets;
mod config;
mod error_handler;
mod init;
mod pages;
mod proxy;
mod request_helper;
mod request_timer;
mod session;
mod state;
mod api;

use config::Configuration;
use state::State;

use crate::request_helper::SqlxMiddleware;

pub fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(500))
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .default_headers(common_headers())
        .build()?)
}

pub fn package_full() -> String {
    format!("{} v{}", package_name(), package_version())
}

pub const fn package_name() -> &'static str {
    const NAME: &str = env!("CARGO_PKG_NAME");
    NAME
}

pub const fn package_version() -> &'static str {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    VERSION
}

fn common_headers() -> HeaderMap {
    let mut hm = HeaderMap::new();
    let user_agent = format!("Mozilla/5.0 ({} v{})", package_name(), package_version());
    trace!("new user agent with value {}", user_agent);
    hm.append(reqwest::header::USER_AGENT, user_agent.parse().unwrap());
    hm
}

async fn run_migrations(config: &Configuration, db_conn: sqlx::Pool<Postgres>) -> Result<()> {
    info!("Migrating database");
    sqlx::migrate!("./migrations").run(&db_conn).await?;
    info!("Database migrated!");
    Ok(())
}

pub struct StatelessPaths {}

impl StatelessPaths {
    pub fn contains(path: &str) -> bool {
        match path {
            "/favicon.svg" => true,
            "/favicon.ico" => true,
            "/robots.txt" => true,
            _ => path.starts_with("/static/") || path.starts_with("/img/"),
        }
    }
}

async fn main_start() -> Result<()> {
    let config = envy::from_env::<Configuration>()?;
    info!("Starting with config {:?}", config);
    let db_conn = config.db_conn().await?;
    run_migrations(&config, db_conn.clone()).await?;
    debug!("Starting job runner");
    let job_runner = app::jobs::runner(db_conn.clone()).await;
    debug!("Configuring application server");
    let mut app = tide::with_state(State::new(config.clone()).await?);
    {
        use tide::utils::After;
        use tide::{Response, StatusCode};
        app.with(request_timer::RequestTimer {});
        app.with(assets::AssetLoader::new(&config)?);
        app.with(SqlxMiddleware::new(db_conn.clone()).await?);
        app.with(state::StateMiddleware {});
        app.with(After(|mut res: Response| async {
            if let Some(err) = res.downcast_error::<async_std::io::Error>() {
                let msg = format!("Error: {:?}", err);
                if let ErrorKind::NotFound = err.kind() {
                    res.set_status(StatusCode::NotFound);
                    res.set_body(msg);
                } else {
                    res.set_status(StatusCode::InternalServerError);
                    res.set_body(msg);
                }
            } else if let Some(err) = res.error() {
                error!("error in request: {}", err);
            }
            Ok(res)
        }));
    }
    app.at("/*").all(proxy::forward);

    app.at("/channels").get(pages::views::channels::html);
    app.at("/static/*").get(assets::serve_asset);
    app.at("/img/:year/:month/:day/:id/:thumbtype")
        .get(pages::image_thumb_get);
    app.at("/favicon.ico").get(assets::serve_topfile);
    app.at("/favicon.svg").get(assets::serve_topfile);
    app.at("/robots.txt").get(assets::serve_topfile);
    app.at("/images/:image_id/fave").post(api::int::image::favorite);
    app.at("/tags/fetch").get(api::int::tag::fetch);
    app.at("/").all(pages::activity_get);
    let scheduler = {
        debug!("Booting up job scheduler");
        let db_conn = db_conn.clone();
        tokio::spawn(async move { app::jobs::scheduler(db_conn).await })
    };
    let server = app.listen(config.listen_on);
    tokio::select! {
        r = server => {
            match r {
                Ok(()) => error!("server exited cleanly but unexpectedly"),
                Err(e) => error!("server error exit: {}", e),
            }
        }
        r = scheduler => {
            match r {
                Ok(()) => error!("scheduler exited cleanly but unexpectedly"),
                Err(e) => error!("scheduler error exit: {}", e),
            }
        }
    }
    drop(job_runner);
    Ok(())
}

fn main() -> Result<()> {
    crate::init::LOGGER.flush();
    use tokio::runtime::Builder;
    {
        for file in assets::Assets::iter() {
            trace!("file in repo: {}", file);
        }
    }
    let runtime = Builder::new_multi_thread()
        .worker_threads(64)
        .max_blocking_threads(64)
        .thread_name_fn(|| {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("tiberius-{}", id)
        })
        .enable_all()
        .build()
        .unwrap();
    info!("Starting {}", package_full());
    runtime.block_on(async move { tokio::spawn(async move { main_start().await }).await? })?;
    runtime.shutdown_timeout(std::time::Duration::from_secs(10));
    Ok(())
}
