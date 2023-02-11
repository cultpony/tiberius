use sqlxmq::{Checkpoint, CurrentJob};
use tiberius_core::{config::Configuration, error::TiberiusResult, session::PostgresSessionStore};
use tiberius_dependencies::prelude::*;
use tiberius_dependencies::sentry;

use crate::SharedCtx;

#[instrument(skip(current_job, sctx))]
#[sqlxmq::job]
pub async fn run_job(current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    sentry::configure_scope(|scope| {
        scope.clear();
    });
    let tx = sentry::start_transaction(sentry::TransactionContext::new("cleanup_sessions", "queue.task"));
    match tx_run_job(current_job, sctx).await {
        Ok(()) => {
            tx.set_status(sentry::protocol::SpanStatus::Ok);
            tx.finish();
            Ok(())
        },
        Err(e) => {
            tx.set_status(sentry::protocol::SpanStatus::InternalError);
            tx.set_data("error_msg", serde_json::Value::String(e.to_string()));
            tx.finish();
            Err(e)
        }
    }
}

#[instrument(skip(current_job, sctx))]
async fn tx_run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    let pool = current_job.pool();
    //let store = todo!();
    //store.cleanup().await?;
    current_job.complete().await?;
    debug!("Job {}: Completed session pruning", current_job.id());
    Ok(())
}
