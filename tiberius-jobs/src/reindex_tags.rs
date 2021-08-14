use rocket::futures::TryStreamExt;
use rocket::Request;
use sqlx::{FromRow, Pool, Postgres};
use sqlxmq::{job, Checkpoint, CurrentJob};
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::state::TiberiusState;
use tiberius_models::{Channel, Client, Tag};
use tracing::{info, trace};

use tiberius_models::Queryable;

use crate::SharedCtx;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TagReindexConfig {
    pub tag_ids: Option<Vec<i64>>,
}

impl Default for TagReindexConfig {
    fn default() -> Self {
        Self { tag_ids: None }
    }
}

#[instrument]
#[sqlxmq::job]
pub async fn run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    info!("Job {}: Reindexing all tags", current_job.id());
    let start = std::time::Instant::now();
    let pool = current_job.pool();
    let progress: TagReindexConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    let mut client = sctx.client;
    match progress.tag_ids {
        None => reindex_all(pool, &mut client).await?,
        Some(v) => reindex_many(&mut client, v).await?,
    }
    info!("Job {}: Reindex complete!", current_job.id());
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

async fn reindex_many(client: &mut Client, ids: Vec<i64>) -> TiberiusResult<()> {
    todo!()
}

async fn reindex_all(pool: &Pool<Postgres>, client: &mut Client) -> TiberiusResult<()> {
    let mut tags = Tag::get_all(pool.clone(), None, None).await?;
    let index_writer = client.index_writer::<Tag>().await?;
    info!("Reindexing all tags, streaming from DB...");
    while let Some(tag) = tags.try_next().await? {
        let tag: Tag = Tag::from_row(&tag)?;
        info!("Reindexing tag {}: {}", tag.id, tag.full_name());
        tag.delete_from_index(index_writer.clone()).await?;
        tag.index(index_writer.clone(), client).await?;
    }
    index_writer.write().await.commit()?;
    Ok(())
}
