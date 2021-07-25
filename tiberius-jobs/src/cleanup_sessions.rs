use log::info;
use sqlxmq::{Checkpoint, CurrentJob};
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::PostgresSessionStore;

#[sqlxmq::job]
pub async fn run_job(mut current_job: CurrentJob) -> TiberiusResult<()> {
    let pool = current_job.pool();
    let store = PostgresSessionStore::from_client(pool.clone());
    store
        .cleanup()
        .await?;
    current_job.complete().await?;
    info!("Job {}: Completed session pruning", current_job.id());
    Ok(())
}
