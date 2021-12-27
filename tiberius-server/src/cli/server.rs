use rocket::{Rocket, Build};
use rocket::yansi::Paint;
use sqlx::Postgres;
use tiberius_core::CSPHeader;
use tiberius_core::app::DBPool;
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::PostgresSessionStore;
use tiberius_core::state::TiberiusState;

use crate::pages;

pub async fn run_migrations(
    _config: &Configuration,
    db_conn: sqlx::Pool<Postgres>,
) -> TiberiusResult<()> {
    info!("Migrating database");
    sqlx::migrate!("../migrations").run(&db_conn).await?;
    info!("Database migrated!");
    Ok(())
}

pub async fn rocket(db_conn: DBPool, config: &Configuration) -> TiberiusResult<Rocket<Build>> {
    let rkt = rocket::build();
    let rkt = rkt.manage(TiberiusState::new(config.clone()).await?);
    let rkt = rkt.attach(CSPHeader);
    let rkt = rkt.manage(PostgresSessionStore::from_client(db_conn.clone()));
    let rkt = rkt.attach(PostgresSessionStore::from_client(db_conn.clone()));

    #[cfg(feature = "full-release")]
    let rkt = rkt.mount(
        "/",
        routes![
            crate::api::int::image::favorite,
            crate::api::int::oembed::fetch,
            crate::api::int::tag::fetch,
            crate::api::v3::images::change_image_uploader,
            crate::api::v3::images::change_image_uploader_user,
            crate::api::v3::misc::sessho::session_handover_user,
            crate::pages::apikeys::manage_keys_page,
            crate::pages::apikeys::create_api_key,
            crate::pages::apikeys::delete_api_key,
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

    #[cfg(feature = "stable-release")]
    let rkt = rkt.mount(
        "/",
        routes![
            crate::api::v3::images::change_image_uploader,
            crate::api::v3::images::change_image_uploader_user,
            crate::api::v3::misc::sessho::session_handover_user,
            crate::pages::apikeys::manage_keys_page,
            crate::pages::apikeys::create_api_key,
            crate::pages::apikeys::delete_api_key,
            crate::pages::session::destroy_session,
            crate::pages::session::new_session_post,
            crate::pages::session::new_session,
            crate::pages::session::registration,
            crate::pages::session::alt_url_new_session_post,
            crate::pages::session::alt_url_new_session,
            crate::pages::session::alt_url_destroy_session,
            crate::pages::channels::list_channels,
            crate::pages::channels::read,
            crate::pages::channels::set_nsfw,
            tiberius_core::assets::serve_asset,
            tiberius_core::assets::serve_favicon_ico,
            tiberius_core::assets::serve_favicon_svg,
            tiberius_core::assets::serve_robots,
        ],
    );

    let rkt = rkt.register("/", catchers![pages::errors::server_error, pages::errors::access_denied]);

    Ok(rkt)
}

pub async fn server_start(start_job_scheduler: bool) -> TiberiusResult<()> {
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

    let rkt = rocket(db_conn.clone(), &config).await?;

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
