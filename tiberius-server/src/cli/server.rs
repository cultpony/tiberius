use std::collections::BTreeMap;

use axum::{
    handler::{Handler, HandlerWithoutStateExt},
    http::{Request, StatusCode},
    middleware, Extension, Router, error_handling::HandleErrorLayer, BoxError,
};
use axum_extra::routing::TypedPath;
use sentry::{Breadcrumb, TransactionContext};
use serde_json::{json, Value};
use sqlx::Postgres;
use tiberius_core::{
    app::DBPool,
    config::Configuration,
    csp_header,
    error::TiberiusResult,
    session::{PostgresSessionStore, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState, UrlDirections},
    CSPHeader, TIBERIUS_SESSION_CACHE_SIZE,
};
use tiberius_dependencies::{
    axum_sessions_auth, sentry,
    tower::ServiceBuilder, tower_sessions::SessionManagerLayer, time::Duration,
};
use tiberius_models::{Client, User};
use tower_cookies::CookieManagerLayer;

use crate::templates::{self, not_found_page, session::PathSessionsLogin};

pub async fn run_migrations(
    _config: &Configuration,
    db_conn: sqlx::Pool<Postgres>,
) -> TiberiusResult<()> {
    info!("Migrating database");
    sqlx::migrate!("../migrations").run(&db_conn).await?;
    info!("Database migrated!");
    Ok(())
}

pub fn setup_all_routes(router: Router<TiberiusState>) -> Router<TiberiusState> {
    let router = crate::api::int::setup_api_int(router);
    let router = crate::api::well_known::setup_well_known(router);
    let router = templates::activity::activity_pages(router);
    let router = templates::apikeys::api_key_pages(router);
    let router = templates::images::image_pages(router);
    let router = templates::channels::channel_pages(router);
    let router = templates::session::session_pages(router);
    let router = templates::static_file_pages(router);
    let router = templates::tags::tags_pages(router);
    let router = templates::filters::setup_filters(router);

    tiberius_core::assets::embedded_file_pages(router)
}

pub async fn axum_setup(db_conn: DBPool, config: &Configuration) -> TiberiusResult<axum::Router> {
    let router = Router::new();

    // TODO: store in config
    let flash_key = axum_flash::Key::generate();
    let csrf_config = axum_csrf::CsrfConfig::default();

    let state = TiberiusState::new(
        config.clone(),
        UrlDirections {
            login_page: PathSessionsLogin {}.to_uri(),
        },
        csrf_config,
        axum_flash::Config::new(flash_key)
            .use_secure_cookies(true /* TODO: determine HTTPS protocol here */),
        CSPHeader {
            static_host: config.cdn_host.clone(),
            camo_host: config.camo_config().map(|(host, _)| host.clone()),
        },
    )
    .await?;

    let router = setup_all_routes(router);

    use tiberius_dependencies::{axum_csrf, tower_sessions, axum_flash};

    let db_store = tower_sessions::PostgresStore::new(db_conn);

    // do session migration
    db_store.migrate().await?;

    let deletion_task = tokio::task::spawn(db_store.clone().continuously_delete_expired(tokio::time::Duration::from_secs(180)));

    let moka_store = tower_sessions::MokaStore::new(TIBERIUS_SESSION_CACHE_SIZE);
    let session_store = tower_sessions::CachingSessionStore::new(moka_store, db_store);
    let session_service = SessionManagerLayer::new(session_store)
      .with_max_age(Duration::days(365))
      .with_same_site(axum_csrf::SameSite::Strict);
    let session_service = ServiceBuilder::new().layer(session_service);

    

    let router = router.layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                StatusCode::BAD_REQUEST
            }))
            .layer(session_service)
            .layer(CookieManagerLayer::new())
            .layer(tiberius_dependencies::sentry_tower::NewSentryLayer::new_from_top())
            .layer(tiberius_dependencies::sentry_tower::SentryHttpLayer::with_transaction()),
    );

    let router = router.route_layer(middleware::from_fn_with_state(state.clone(), csp_header));

    let router = router.fallback(not_found_page);

    let router = router.with_state::<()>(state);

    Ok(router)
}

pub async fn server_start(
    start_job_scheduler: bool,
    config: Configuration,
) -> TiberiusResult<()> {
    info!("Starting with config {:?}", config);
    let db_conn: DBPool = config.db_conn().await?;
    run_migrations(&config, db_conn.clone()).await?;
    debug!("Configuring application server");

    let axum = axum_setup(db_conn.clone(), &config).await?;

    if config.rebuild_index_on_startup {
        warn!("Rebuilding search index due to --rebuild-index-on-startup");
        let db_conn_c = db_conn.clone();
        let mut client = Client::new(db_conn_c, config.search_dir.as_ref());
        tiberius_jobs::reindex_images::reindex_all(&mut client).await?;
        tiberius_jobs::reindex_tags::reindex_all(&mut client).await?;
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

#[cfg(test)]
mod test {
    #[test]
    pub fn test_verify_routes_build() {
        let router = axum::Router::new();

        let _ = std::hint::black_box(super::setup_all_routes(router));
    }
}
