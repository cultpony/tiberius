use std::collections::BTreeMap;

use axum::{handler::Handler, http::Request, Extension, Router};
use axum_extra::routing::TypedPath;
use sentry::{Breadcrumb, TransactionContext};
use serde_json::{json, Value};
use sqlx::Postgres;
use tiberius_core::{
    app::DBPool,
    config::Configuration,
    error::TiberiusResult,
    session::{PostgresSessionStore, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState, UrlDirections},
    CSPHeader,
};
use tiberius_dependencies::{
    axum_database_sessions::{AxumPgPool, AxumSessionConfig, AxumSessionStore},
    axum_sessions_auth, sentry,
    tower::ServiceBuilder,
};
use tiberius_models::{Client, User};
use tower_cookies::CookieManagerLayer;

use crate::pages::{self, not_found_page, session::PathSessionsLogin};

pub async fn run_migrations(
    _config: &Configuration,
    db_conn: sqlx::Pool<Postgres>,
) -> TiberiusResult<()> {
    info!("Migrating database");
    sqlx::migrate!("../migrations").run(&db_conn).await?;
    info!("Database migrated!");
    Ok(())
}

pub async fn axum_setup(db_conn: DBPool, config: &Configuration) -> TiberiusResult<axum::Router> {
    let router = Router::new();

    let router = crate::api::int::setup_api_int(router);
    let router = crate::api::well_known::setup_well_known(router);
    let router = pages::activity::activity_pages(router);
    let router = pages::apikeys::api_key_pages(router);
    let router = pages::images::image_pages(router);
    let router = pages::channels::channel_pages(router);
    let router = pages::session::session_pages(router);
    let router = pages::static_file_pages(router);
    let router = pages::tags::tags_pages(router);
    let router = pages::filters::setup_filters(router);
    let router = tiberius_core::assets::embedded_file_pages(router);

    use tiberius_dependencies::{axum_csrf, axum_database_sessions, axum_flash};

    let axum_session_config = AxumSessionConfig::default()
        .with_table_name("user_sessions")
        .with_cookie_name("tiberius_session");
    let axum_session_store = AxumSessionStore::<axum_database_sessions::AxumPgPool>::new(
        Some(db_conn.clone().into()),
        axum_session_config,
    );
    axum_session_store.initiate().await?;

    // TODO: store in config
    let flash_key = axum_flash::Key::generate();
    let csrf_config = axum_csrf::CsrfConfig::default();

    let router = router.layer(
        ServiceBuilder::new()
            .layer(Extension(
                TiberiusState::new(
                    config.clone(),
                    UrlDirections {
                        login_page: PathSessionsLogin {}.to_uri(),
                    },
                )
                .await?,
            ))
            .layer(Extension(CSPHeader {
                static_host: config.cdn_host.clone(),
                camo_host: config.camo_config().map(|(host, _)| host.clone()),
            }))
            .layer(axum_database_sessions::AxumSessionLayer::new(
                axum_session_store,
            ))
            .layer(axum_flash::layer(flash_key).with_cookie_manager())
            .layer(
                axum_csrf::CsrfLayer::new(csrf_config),
            )
            .layer(CookieManagerLayer::new())
            .layer(sentry_tower::NewSentryLayer::new_from_top())
            .layer(sentry_tower::SentryHttpLayer::with_transaction()),
    );

    let router = router.fallback(not_found_page.into_service());

    Ok(router)
}

/*
crate::api::v3::images::change_image_uploader_user,
crate::api::v3::images::change_image_uploader,
crate::api::v3::images::get_image_data,
crate::api::v3::misc::sessho::session_handover_user,
crate::pages::blog::staff_page::add_user_to_category,
crate::pages::blog::staff_page::edit_user_entry,
crate::pages::blog::staff_page::new_category,
crate::pages::blog::staff_page::show,
crate::pages::channels::list_channels,
crate::pages::channels::read,
crate::pages::channels::set_nsfw,
crate::pages::session::alt_url_destroy_session,
crate::pages::session::alt_url_new_session_post,
crate::pages::session::alt_url_new_session,
crate::pages::session::destroy_session,
crate::pages::session::new_session_post,
crate::pages::session::new_session,
crate::pages::session::registration,
tiberius_core::assets::serve_asset,
tiberius_core::assets::serve_favicon_ico,
tiberius_core::assets::serve_favicon_svg,
tiberius_core::assets::serve_robots,
*/
pub async fn server_start(
    start_job_scheduler: bool,
    start_jobs: bool,
    config: Configuration,
) -> TiberiusResult<()> {
    info!("Starting with config {:?}", config);
    let db_conn: DBPool = config.db_conn().await?;
    run_migrations(&config, db_conn.clone()).await?;
    let job_runner = if start_jobs {
        debug!("Starting job runner");
        Some(tiberius_jobs::runner(db_conn.clone(), config.clone()))
    } else {
        None
    };
    debug!("Configuring application server");

    let axum = axum_setup(db_conn.clone(), &config).await?;

    if config.rebuild_index_on_startup {
        warn!("Rebuilding search index due to --rebuild-index-on-startup");
        let db_conn_c = db_conn.clone();
        let mut client = Client::new(db_conn_c, config.search_dir.as_ref());
        tiberius_jobs::reindex_images::reindex_all(&db_conn, &mut client).await?;
        tiberius_jobs::reindex_tags::reindex_all(&db_conn, &mut client).await?;
        warn!("Index Rebuild complete");
    }

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
    let server = axum.into_make_service();
    let server = axum::Server::bind(&config.bind_to).serve(server);
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
                error!("Could not start server: {:?}", e);
            }
        }
    }
    println!("Tiberius exited.");
    Ok(())
}
