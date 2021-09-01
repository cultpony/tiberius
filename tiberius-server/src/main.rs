//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate tracing;

use std::{path::Path, str::FromStr};

use tracing::{debug, info};

use rocket::yansi::Paint;
use sqlx::Postgres;
use tiberius_core::app::DBPool;
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::PostgresSessionStore;
use tiberius_core::state::TiberiusState;
use tiberius_core::{package_full, package_name, package_version, CSPHeader};

mod api;
mod init;
mod pages;

const MAX_IMAGE_DIMENSION: u32 = 2_000_000u32;

async fn run_migrations(
    _config: &Configuration,
    db_conn: sqlx::Pool<Postgres>,
) -> TiberiusResult<()> {
    info!("Migrating database");
    sqlx::migrate!("../migrations").run(&db_conn).await?;
    info!("Database migrated!");
    Ok(())
}

async fn server_start(start_job_scheduler: bool) -> TiberiusResult<()> {
    let config = envy::from_env::<Configuration>()?;
    info!("Starting with config {:?}", config);
    let db_conn: DBPool = config.db_conn().await?;
    run_migrations(&config, db_conn.clone()).await?;
    let job_runner = if start_job_scheduler {
        debug!("Starting job runner");
        Some(tiberius_jobs::runner(db_conn.clone(), config.clone()))
    } else {
        None
    };
    debug!("Configuring application server");

    let rkt = rocket::build();
    let rkt = rkt.manage(TiberiusState::new(config.clone()).await?);
    let rkt = rkt.attach(CSPHeader);
    let rkt = rkt.manage(PostgresSessionStore::from_client(db_conn.clone()));
    let rkt = rkt.attach(PostgresSessionStore::from_client(db_conn.clone()));
    let rkt = rkt.mount(
        "/",
        routes![
            crate::api::int::image::favorite,
            crate::api::int::oembed::fetch,
            crate::api::int::tag::fetch,
            crate::api::v3::images::change_image_uploader,
            crate::api::well_known::imageboard_type::imageboardapiflavor_philomena_int,
            crate::api::well_known::imageboard_type::imageboardapiflavor_philomena_v1,
            crate::api::well_known::imageboard_type::imageboardapiflavor,
            crate::api::well_known::imageboard_type::imageboardtype,
            tiberius_core::assets::serve_asset,
            tiberius_core::assets::serve_favicon_ico,
            tiberius_core::assets::serve_favicon_svg,
            tiberius_core::assets::serve_robots,
            crate::pages::activity::index,
            crate::pages::channels::list_channels,
            crate::pages::channels::read,
            crate::pages::channels::set_nsfw,
            crate::pages::files::image_full_get,
            crate::pages::files::image_thumb_get_simple,
            crate::pages::files::image_thumb_get,
            crate::pages::images::new_image,
            crate::pages::images::show_image,
            crate::pages::images::upload_image,
            crate::pages::images::search_empty,
            crate::pages::images::search_reverse_page,
            crate::pages::images::search,
            crate::pages::session::destroy_session,
            crate::pages::session::new_session_post,
            crate::pages::session::new_session_totp,
            crate::pages::session::new_session,
            crate::pages::session::registration,
            crate::pages::tags::alias,
            crate::pages::tags::edit_tag,
            crate::pages::tags::list_tags,
            crate::pages::tags::reindex,
            crate::pages::tags::show_tag,
            crate::pages::tags::tag_changes,
            crate::pages::tags::usage,
            crate::pages::tags::autocomplete,
        ],
    );

    let rkt = rkt.register("/", catchers![pages::errors::server_error]);
    let scheduler = if start_job_scheduler {
        debug!("Booting up job scheduler");
        let db_conn = db_conn.clone();
        let config = config.clone();
        Some(tokio::spawn(async move {
            tiberius_jobs::scheduler(db_conn, config).await
        }))
    } else {
        None
    };
    let server = rkt.launch();
    if start_job_scheduler {
        let scheduler = scheduler.unwrap();
        let job_runner = job_runner.unwrap();
        tokio::select! {
            r = server => {
                match r {
                    Ok(()) => error!("server exited cleanly but unexpectedly"),
                    Err(e) => error!("server error exit: {:?}", e),
                }
            }
            r = scheduler => {
                match r {
                    Ok(()) => error!("scheduler exited cleanly but unexpectedly"),
                    Err(e) => error!("scheduler error exit: {}", e),
                }
            }
            r = job_runner => {
                match r {
                    Ok(()) => error!("job runner exited cleanly but unexpectedly"),
                    Err(e) => error!("scheduler error exit: {}", e),
                }
            }
        };
    } else {
        match server.await {
            Ok(()) => error!("server exited cleanly but unexpectedly"),
            Err(e) => {
                error!("Could not start server: {}", e);
                match e.kind() {
                    rocket::error::ErrorKind::Collisions(v) => {
                        for &(ref a, ref b) in &v.catchers {
                            info!("{} {} {}", a, Paint::red("collision").italic(), b);
                        }
                        for &(ref a, ref b) in &v.routes {
                            info!("{} {} {}", a, Paint::red("collision").italic(), b);
                        }
                    }
                    v => error!("{:?}", v),
                }
            }
        }
    }
    println!("Tiberius exited.");
    Ok(())
}

fn main() -> TiberiusResult<()> {
    crate::init::LOGGER.flush();
    use tokio::runtime::Builder;
    let runtime = Builder::new_multi_thread()
        .worker_threads(16)
        .max_blocking_threads(16)
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
        .subcommand(
            SubCommand::with_name("server")
                .about("starts main server")
                .arg(
                    Arg::with_name("no-jobs")
                        .long("no-jobs")
                        .short("z")
                        .help("disable job scheduling and running"),
                ),
        )
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

    if let Some(matches) = matches.subcommand_matches("server") {
        info!("Starting {}", package_full());
        let job_runner = !matches.is_present("no-jobs");
        if !job_runner {
            warn!("Running without job scheduler and job runner");
        }
        runtime.block_on(async move {
            tokio::spawn(async move { server_start(job_runner).await }).await
        })??;
        runtime.shutdown_timeout(std::time::Duration::from_secs(10));
        Ok(())
    } else if let Some(matches) = matches.subcommand_matches("verify-db") {
        runtime.block_on(async move {
            let table = matches.value_of("table");
            let table = match table {
                None => {
                    error!("require to know which table to verify");
                    return Ok(());
                }
                Some(t) => t,
            };
            let start_id = matches.value_of("start-id");
            let start_id: u64 = match start_id {
                None => {
                    error!("require to know where to start verify");
                    return Ok(());
                }
                Some(t) => t.parse().expect("can't parse start id"),
            };
            let stop_id = matches.value_of("stop-id");
            let stop_id: u64 = match stop_id {
                None => {
                    error!("require to know where to stop verify");
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
            use tiberius_models::*;
            let client = Client::new(db_conn.clone(), &config.search_dir);
            let mut table: Box<dyn tiberius_models::VerifiableTable> = match table {
                "images" => Image::verifier(client, db_conn, start_id, stop_id, 10240),
                v => {
                    error!("table {} is invalid", v);
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
            info!("Creating keys directory...");
            std::fs::create_dir_all(&base_path)?;
        }
        let rng = ring::rand::SystemRandom::new();
        info!("Generting keys...");
        let ed25519path = base_path.join(Path::new("ed25519.pkcs8"));
        let mainkeypath = base_path.join(Path::new("main.key"));

        let sessionkeypath = base_path.join(Path::new("session.key"));
        if !ed25519path.exists() {
            info!("Generating signing key");
            let signing_key = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)?;
            std::fs::write(ed25519path, signing_key.as_ref())?;
        }
        if !mainkeypath.exists() {
            info!("Generating main key");
            let random_key: [u8; 64] = ring::rand::generate(&rng)?.expose();
            //TODO: generate other needed keys
            std::fs::write(mainkeypath, random_key.as_ref())?;
        }
        if !sessionkeypath.exists() {
            info!("Generating session key");
            let random_key: [u8; 64] = ring::rand::generate(&rng)?.expose();
            std::fs::write(sessionkeypath, random_key.as_ref())?;
        }
        warn!("Keys generated, you are ready to roll.");
        error!("MAKE BACKUPS OF THE {} DIRECTORY", base_path.display());
        Ok(())
    } else {
        error!("No subcommand specified, please tell me what to do or use --help");
        Ok(())
    }
}
