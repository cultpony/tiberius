use rocket::http::Status;
use rocket::local::asynchronous::Client;
use tiberius_core::app::DBPool;
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use rocket::http::Header;

use crate::cli::server::rocket;
use crate::api::v3::images::ChangeUploader;

#[cfg(any(feature = "full-release", feature = "stable-release"))]
#[sqlx_database_tester::test(
    pool(variable = "pool", migrations = "../migrations"),
)]
async fn test_staff_only_mode_enabled() -> TiberiusResult<()> {
    let mut config = Configuration::default();
    unsafe { config.set_staff_key(Some("test".to_string())) };
    unsafe { config.set_alt_dbconn(pool.clone()) };
    let rocket = rocket(pool, &config).await.unwrap();
    let client = Client::tracked(rocket).await.unwrap();

    let resp = client.get("/sessions/login")
        .header(Header::new("X-Tiberius-Staff-Auth", "test"))
        .dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "Must accept all access to sessions with staff key if staff key configured");

    let resp = client.get("/sessions/login")
        .dispatch().await;
    assert_eq!(resp.status(), Status::Forbidden, "Must deny all access to sessions without staff key if staff key configured");
    Ok(())
}

#[cfg(any(feature = "full-release", feature = "stable-release"))]
#[sqlx_database_tester::test(
    pool(variable = "pool", migrations = "../migrations"),
)]
async fn test_staff_only_mode_disabled() -> TiberiusResult<()> {
    let mut config = Configuration::default();
    unsafe { config.set_staff_key(None) };
    unsafe { config.set_alt_dbconn(pool.clone()) };
    let rocket = rocket(pool, &config).await.unwrap();
    let client = Client::tracked(rocket).await.unwrap();

    let resp = client.get("/sessions/login")
        .dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "Must accept unauthenticated access to pages without staff key if no staff key configured");

    let resp = client.get("/sessions/login")
        .header(Header::new("X-Tiberius-Staff-Auth", "test"))
        .dispatch().await;
    assert_eq!(resp.status(), Status::Ok, "Must accept all access to sessions with staff key if no staff key configured");
    Ok(())
}