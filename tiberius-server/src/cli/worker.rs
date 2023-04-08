use tiberius_core::app::DBPool;
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;

pub async fn worker_start(config: Configuration) -> TiberiusResult<()> {
    info!("Starting with config {:?}", config);
    let db_conn: DBPool = config.db_conn().await?;
    tiberius_jobs::scheduler(db_conn, config).await
}
