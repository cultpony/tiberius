use futures_util::stream::StreamExt;
use sqlx::{FromRow, Pool, Postgres};
use sqlxmq::{job, Checkpoint, CurrentJob};
use tiberius_core::{config::Configuration, error::TiberiusResult, state::TiberiusState};
use tiberius_models::{Channel, Client, Image, ImageSortBy};
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

#[instrument(level = "trace")]
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

#[instrument(level = "trace")]
async fn reindex_many(client: &mut Client, ids: Vec<i64>) -> TiberiusResult<()> {
    let images = Image::get_many(client, ids, ImageSortBy::Random).await?;
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

#[instrument(level = "trace")]
pub async fn reindex_all(pool: &Pool<Postgres>, client: &mut Client) -> TiberiusResult<()> {
    let image_count = Image::count(&mut Client::new(pool.clone(), None), None, None).await?;
    let mut images = Image::get_all(pool.clone(), None, None).await?;
    let index_writer = client.index_writer::<Image>().await?;
    let index_reader = client.index_reader::<Image>()?;
    info!("Reindexing {image_count} images, streaming from DB...");
    let mut counter = 0;
    let progress_indicator = image_count / 521;
    while let Some(image) = images.next().await.transpose()? {
        if counter % progress_indicator == 0 {
            info!(
                "Progress {:07.3}%",
                (counter as f64 / image_count as f64) * 100.0
            );
            index_writer.write().await.commit()?;
        }
        counter += 1;
        let image: Image = Image::from_row(&image)?;
        let image_in_index =
            Image::get_from_index(index_reader.clone(), image.identifier()).await?;
        let image_in_db = image.get_doc(client, true).await?;
        match image_in_index {
            Some(image_in_index) => {
                if image_in_db != image_in_index {
                    info!("Reindexing image {counter}/{image_count}: {}", image.id);
                } else {
                    debug!(
                        "Image {counter}/{image_count} requires no reindex: {}",
                        image.id
                    );
                    continue;
                }
            }
            None => {
                info!(
                    "Image {counter}/{image_count} indexing for first time: {}",
                    image.id
                );
            }
        }
        image.delete_from_index(index_writer.clone()).await?;
        image.index(index_writer.clone(), client).await?;
    }
    index_writer.write().await.commit()?;
    Ok(())
}
