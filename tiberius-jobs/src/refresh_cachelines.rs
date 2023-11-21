use std::ops::Range;

use crate::SharedCtx;
use crate::scheduler::CurrentJob;
use tiberius_core::error::TiberiusResult;
use tiberius_dependencies::prelude::*;
use tiberius_dependencies::sentry;
use tiberius_dependencies::serde;
use tiberius_dependencies::serde_json;
use tiberius_models::Image;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct RefreshCachelineConfig {
    pub image_id_range: Range<u64>,
}

#[instrument(skip(current_job, sctx))]
pub async fn run_job(current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    sentry::configure_scope(|scope| {
        scope.clear();
    });
    let tx = sentry::start_transaction(sentry::TransactionContext::new(
        "refresh_cachelines",
        "queue.task",
    ));
    match tx_run_job(current_job, sctx).await {
        Ok(()) => {
            tx.set_status(sentry::protocol::SpanStatus::Ok);
            tx.finish();
            Ok(())
        }
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
    debug!("Job {}: Refreshing Cachelines", current_job.id());
    let start = std::time::Instant::now();
    let mut progress: RefreshCachelineConfig = current_job
        .data()?
        .expect("job requires configuration copy");
    let mut client = sctx.client();
    let imgs = Image::get_range(&mut client, progress.image_id_range.clone()).await?;
    for mut image in imgs {
        debug!("Processing image {}", image.id);
        if image.update_cache_lines(&mut client).await? {
            let id = image.id;
            debug!("Updating image {}", id);
            image.save(&mut client).await?;
            progress.image_id_range = (id as u64)..(progress.image_id_range.end);
        }
    }
    let end = std::time::Instant::now();
    let time_spent = end - start;
    let time_spent = time_spent.as_secs_f32();
    debug!(
        "Job {}: Processing complete in {:4.3} seconds!",
        current_job.id(),
        time_spent
    );
    Ok(())
}
