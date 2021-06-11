use anyhow::{Context, Result};
use log::info;
use sqlxmq::{job, Checkpoint, CurrentJob};

use crate::session::PostgresSessionStore;

#[job]
pub async fn run_job(mut current_job: CurrentJob) -> Result<()> {
    let pool = current_job.pool();
    let store = PostgresSessionStore::from_client(pool.clone());
    store
        .cleanup()
        .await
        .context("cleanup in database failed")?;
    current_job.complete().await?;
    info!("Job {}: Completed session pruning", current_job.id());
    Ok(())
}
