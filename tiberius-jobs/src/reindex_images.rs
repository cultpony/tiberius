use rocket::futures::TryStreamExt;
use rocket::Request;
use sqlx::{FromRow, Pool, Postgres};
use sqlxmq::{job, Checkpoint, CurrentJob};
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::state::TiberiusState;
use tiberius_models::{Channel, Client, Image};
use tracing::{info, trace};

use tiberius_models::Queryable;

use crate::SharedCtx;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ImageReindexConfig {
    pub image_ids: Option<Vec<i64>>,
}

impl Default for ImageReindexConfig {
    fn default() -> Self {
        Self { image_ids: None }
    }
}

pub async fn reindex_images<'a, E: sqlx::Executor<'a, Database = sqlx::Postgres>>(
    executor: E,
    ipc: ImageReindexConfig,
) -> TiberiusResult<()> {
    run_job.builder().set_json(&ipc)?.spawn(executor).await?;
    Ok(())
}

#[instrument]
#[sqlxmq::job]
pub async fn run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    info!("Job {}: Reindexing all images", current_job.id());
    let start = std::time::Instant::now();
    let pool = current_job.pool();
    let progress: ImageReindexConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    let mut client = sctx.client;
    match progress.image_ids {
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
    let images = Image::get_many(client, ids).await?;
    let index_writer = client.index_writer::<Image>().await?;
    info!("Reindexing all images, streaming from DB...");
    for image in images {
        info!("Reindexing image {} {:?}", image.id, image.image);
        image.delete_from_index(index_writer.clone()).await?;
        image.index(index_writer.clone(), client).await?;
    }
    let mut index_writer = index_writer.write().await;
    index_writer.commit()?;
    drop(index_writer);
    Ok(())
}

async fn reindex_all(pool: &Pool<Postgres>, client: &mut Client) -> TiberiusResult<()> {
    let mut images = Image::get_all(pool.clone(), None, None).await?;
    let index_writer = client.index_writer::<Image>().await?;
    info!("Reindexing all images, streaming from DB...");
    while let Some(image) = images.try_next().await? {
        let image: Image = Image::from_row(&image)?;
        info!("Reindexing image {} {:?}", image.id, image.image);
        image.delete_from_index(index_writer.clone()).await?;
        image.index(index_writer.clone(), client).await?;
    }
    index_writer.write().await.commit()?;
    Ok(())
}
