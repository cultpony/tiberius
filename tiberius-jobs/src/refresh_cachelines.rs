use std::ops::Range;

use crate::SharedCtx;
use sqlxmq::{Checkpoint, CurrentJob};
use tiberius_core::error::TiberiusResult;
use tiberius_models::Image;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct RefreshCachelineConfig {
    pub image_id_range: Range<u64>,
}

#[instrument(level = "trace")]
#[sqlxmq::job(retries = 100, backoff_secs = 10)]
pub async fn run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    info!("Job {}: Refreshing Cachelines", current_job.id());
    let start = std::time::Instant::now();
    let pool = current_job.pool();
    let mut progress: RefreshCachelineConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    let mut client = sctx.client;
    let imgs = Image::get_range(&mut client, progress.image_id_range.clone()).await?;
    for mut image in imgs {
        debug!("Processing image {}", image.id);
        if image.update_cache_lines(&mut client).await? {
            let id = image.id;
            info!("Updating image {}", id);
            image.save(&mut client).await?;
            progress.image_id_range = (id as u64)..(progress.image_id_range.end);
            let mut checkpoint = Checkpoint::new();
            checkpoint.set_json(&progress)?;
            current_job.checkpoint(&checkpoint).await?;
        }
    }
    current_job.complete().await?;
    let end = std::time::Instant::now();
    let time_spent = end - start;
    let time_spent = time_spent.as_secs_f32();
    info!(
        "Job {}: Processing complete in {:4.3} seconds!",
        current_job.id(),
        time_spent
    );
    Ok(())
}
