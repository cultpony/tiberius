use log::info;
use sqlxmq::{job, Checkpoint, CurrentJob};

use crate::{error::TiberiusResult, session::PostgresSessionStore};

#[job]
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
