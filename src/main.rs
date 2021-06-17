#[macro_use] extern crate rocket;

use std::{io::ErrorKind, path::Path, str::FromStr, time::Duration};

use log::{debug, info, trace};
use reqwest::{header::HeaderMap, Proxy};
use sqlx::Postgres;

mod api;
mod app;
mod assets;
mod config;
mod init;
mod error;
mod pages;
//mod proxy;
mod request_helper;
mod session;
mod state;

use config::Configuration;
use state::State;

use crate::{app::{DBPool, common::CSPHeader}, assets::AssetLoader, error::TiberiusResult, pages::{todo_page, todo_page_fn, views}, request_helper::SqlxMiddleware, session::PostgresSessionStore};

pub fn http_client(config: &Configuration) -> TiberiusResult<reqwest::Client> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(500))
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none());
    let client = if let Some(proxy) = &config.proxy {
        client.proxy(Proxy::all(proxy.clone())?)
    } else {
        client
    };
    Ok(client.default_headers(common_headers()).build()?)
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

async fn run_migrations(config: &Configuration, db_conn: sqlx::Pool<Postgres>) -> TiberiusResult<()> {
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

async fn server_start() -> TiberiusResult<()> {
    let config = envy::from_env::<Configuration>()?;
    info!("Starting with config {:?}", config);
    let db_conn: DBPool = config.db_conn().await?;
    run_migrations(&config, db_conn.clone()).await?;
    debug!("Starting job runner");
    let job_runner = app::jobs::runner(db_conn.clone()).await;
    debug!("Configuring application server");

    let rkt = rocket::build();
    let rkt = rkt.manage(State::new(config.clone()).await?);
    let rkt = rkt.manage(db_conn.clone());
    let rkt = rkt.manage(config.clone());
    let rkt = rkt.attach(CSPHeader);
    let rkt = rkt.attach(AssetLoader::new(&config)?);
    let rkt = rkt.attach(PostgresSessionStore::from_client(db_conn.clone()));
    let rkt = rkt.mount("/", routes![
        crate::pages::channels::list_channels,
        crate::pages::channels::set_nsfw,
    ]);
    /*let mut app = tide::with_state(State::new(config.clone()).await?);
    {
        use tide::utils::After;
        use tide::{Response, StatusCode};
        app.with(request_timer::RequestTimer {});
        app.with(assets::AssetLoader::new(&config)?);
        app.with(SqlxMiddleware::new(db_conn.clone()).await?);
        let session_store = PostgresSessionStore::from_client(db_conn.clone());
        let session_key = std::fs::read(config.key_directory.join(Path::new("session.key")))?;
        app.with(
            tide::sessions::SessionMiddleware::new(session_store, &session_key)
                .with_cookie_name(package_name())
                .with_same_site_policy(tide::http::cookies::SameSite::Strict)
                .with_session_ttl(Some(Duration::from_secs(10 * 60)))
                .without_save_unchanged(),
        );
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
    app.at("").all(proxy::forward);

    use tide_fluent_routes::prelude::*;
    let routes = root()
        .at("/", |r| r.get(pages::activity::activity_get))
        .at("", |route| route.all(proxy::forward))
        .at("/api", |route| {
            route.at("/v1/json", |route| route.all(todo_page_fn))
        })
        .at("/channels", |route| {
            route
                .at("/subscription", |r| r.all(todo_page_fn))
        })
        .at("/:image_id", |r| r.get(todo_page_fn))
        .at("/images", |route| {
            route
                .at("/:image_id", |route| {
                    route
                        .get(todo_page_fn)
                        .at("/vote", |r| r.get(todo_page_fn))
                        .at("/fave", |r| r.get(todo_page_fn))
                        .at("/hide", |r| r.get(todo_page_fn))
                })
                .at("/scrape", |r| r.get(todo_page_fn))
                .at("/random", |r| r.get(todo_page_fn))
        })
        .at("/tags", |route| {
            route.get(pages::tags::list_tags).at("/:tag", |route| {
                route
                    .get(pages::tags::show_tag)
                    .at("/fetch", |r| r.get(pages::tags::show_tag))
                    .at("/image", |r| r.all(todo_page_fn))
                    .at("/alias", |r| r.all(todo_page_fn))
                    .at("/reindex", |r| r.all(todo_page_fn))
            })
        })
        .at("/sessions", |route| {
            route
                .at("/register", |r| r.get(pages::session::registration))
                .at("/login", |r| {
                    r.get(pages::session::new_session)
                        .post(pages::session::new_session_post)
                })
                .at("/totp", |r| r.get(pages::session::new_session_totp))
                .at("/logout", |r| r.get(pages::session::destroy_session))
        })
        .at("/static", |r| r.get(assets::serve_asset))
        .at("/img/", |route| {
            route.at("/:year/:month/:day/:id/:thumbtype", |r| {
                r.get(pages::image_thumb_get)
            })
        })
        .at("/favicon.ico", |r| r.get(assets::serve_topfile))
        .at("/favicon.svg", |r| r.get(assets::serve_topfile))
        .at("/robots.txt", |r| r.get(assets::serve_topfile));

    app.register(routes);
    */

    let scheduler = {
        debug!("Booting up job scheduler");
        let db_conn = db_conn.clone();
        let config = config.clone();
        tokio::spawn(async move { app::jobs::scheduler(db_conn, config).await })
    };
    let server = rkt.launch();
    //let server = app.listen(config.listen_on);
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

fn main() -> TiberiusResult<()> {
    crate::init::LOGGER.flush();
    use tokio::runtime::Builder;
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

    use clap::{App, Arg, SubCommand};
    let app = App::new(package_name())
        .version(package_version())
        .about("The Lunar Image Board")
        .subcommand(SubCommand::with_name("server").about("starts main server"))
        .subcommand(
            SubCommand::with_name("verify-db")
                .about("verify database-integrity")
                .arg(
                    Arg::with_name("table")
                        .short("t")
                        .required(true)
                        .takes_value(true)
                        .help("table to verify"),
                )
                .arg(
                    Arg::with_name("start-id")
                        .short("s")
                        .required(true)
                        .takes_value(true)
                        .help("ID to start at"),
                )
                .arg(
                    Arg::with_name("stop-id")
                        .short("e")
                        .required(true)
                        .takes_value(true)
                        .help("ID to stop at"),
                ),
        )
        .subcommand(
            SubCommand::with_name("gen-keys")
                .about("generate or refresh server cryptographic keys")
                .arg(
                    Arg::with_name("key-directory")
                        .help("key directory to use")
                        .index(1)
                        .takes_value(true)
                        .required(true),
                ),
        );

    let matches = app.get_matches();

    if let Some(_) = matches.subcommand_matches("server") {
        {
            for file in assets::Assets::iter() {
                trace!("file in repo: {}", file);
            }
        }
        info!("Starting {}", package_full());
        runtime
            .block_on(async move { tokio::spawn(async move { server_start().await }).await? })?;
        runtime.shutdown_timeout(std::time::Duration::from_secs(10));
        Ok(())
    } else if let Some(matches) = matches.subcommand_matches("verify-db") {
        runtime.block_on(async move {
            let table = matches.value_of("table");
            let table = match table {
                None => {
                    log::error!("require to know which table to verify");
                    return Ok(());
                }
                Some(t) => t,
            };
            let start_id = matches.value_of("start-id");
            let start_id: u64 = match start_id {
                None => {
                    log::error!("require to know where to start verify");
                    return Ok(());
                }
                Some(t) => t.parse().expect("can't parse start id"),
            };
            let stop_id = matches.value_of("stop-id");
            let stop_id: u64 = match stop_id {
                None => {
                    log::error!("require to know where to stop verify");
                    return Ok(());
                }
                Some(t) => t.parse().expect("can't parse stop id"),
            };
            let config = envy::from_env::<Configuration>().expect("could not parse config");
            info!("Starting with config {:?}", config);
            let db_conn = config
                .db_conn()
                .await
                .expect("could not establish db connection");
            use philomena_models::*;
            let client = Client::new(
                db_conn.clone(),
                &config.search_dir,
            );
            let mut table: Box<dyn philomena_models::VerifiableTable> = match table {
                "images" => Image::verifier(client, db_conn, start_id, stop_id, 10240),
                v => {
                    log::error!("table {} is invalid", v);
                    return Ok(());
                }
            };
            table.verify().await
        })?;
        Ok(())
    } else if let Some(matches) = matches.subcommand_matches("gen-keys") {
        let base_path = matches
            .value_of("key-directory")
            .expect("must have key directory");
        let base_path = std::path::PathBuf::from_str(base_path)?;
        if !base_path.exists() {
            log::info!("Creating keys directory...");
            std::fs::create_dir_all(&base_path)?;
        }
        let rng = ring::rand::SystemRandom::new();
        log::info!("Generting keys...");
        let ed25519path = base_path.join(Path::new("ed25519.pkcs8"));
        let mainkeypath = base_path.join(Path::new("main.key"));

        let sessionkeypath = base_path.join(Path::new("session.key"));
        if !ed25519path.exists() {
            log::info!("Generating signing key");
            let signing_key = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)?;
            std::fs::write(ed25519path, signing_key.as_ref())?;
        }
        if !mainkeypath.exists() {
            log::info!("Generating main key");
            let random_key: [u8; 64] = ring::rand::generate(&rng)?.expose();
            //TODO: generate other needed keys
            std::fs::write(mainkeypath, random_key.as_ref())?;
        }
        if !sessionkeypath.exists() {
            log::info!("Generating session key");
            let random_key: [u8; 64] = ring::rand::generate(&rng)?.expose();
            std::fs::write(sessionkeypath, random_key.as_ref())?;
        }
        log::warn!("Keys generated, you are ready to roll.");
        log::error!("MAKE BACKUPS OF THE {} DIRECTORY", base_path.display());
        Ok(())
    } else {
        log::error!("No subcommand specified, please tell me what to do or use --help");
        Ok(())
    }
}
