use sqlxmq::{Checkpoint, CurrentJob};
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::PostgresSessionStore;
use tracing::info;

use crate::SharedCtx;

#[instrument]
#[sqlxmq::job]
pub async fn run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    let pool = current_job.pool();
    let store = PostgresSessionStore::from_client(pool.clone());
    store.cleanup().await?;
    current_job.complete().await?;
    info!("Job {}: Completed session pruning", current_job.id());
    Ok(())
}
