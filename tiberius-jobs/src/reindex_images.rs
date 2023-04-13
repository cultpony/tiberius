use tiberius_core::{config::Configuration, error::TiberiusResult, state::TiberiusState};
use tiberius_dependencies::futures_util::stream::StreamExt;
use tiberius_dependencies::itertools::Itertools;
use tiberius_dependencies::prelude::*;
use tiberius_dependencies::sentry;
use tiberius_dependencies::serde;
use tiberius_dependencies::serde_json;
use tiberius_dependencies::sqlx;
use tiberius_dependencies::sqlx::{FromRow, Pool, Postgres};
use tiberius_dependencies::sqlxmq;
use tiberius_dependencies::sqlxmq::{job, Checkpoint, CurrentJob};
use tiberius_models::{Channel, Client, Image, ImageSortBy};

use tiberius_models::Queryable;

use crate::SharedCtx;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct ImageReindexConfig {
    /// If none and only_new is false, all images are reindexed
    /// If none and only_new is true, only new images are reindex
    /// If some then listed images are indexed in addition to new images if only_new is true
    pub image_ids: Option<Vec<i64>>,
    /// If set true, only images that are newer than the latest indexed image are reindexed
    /// in addition to all images listed in the image_ids list
    ///
    /// If no image IDs are listed, this will result in indexing only new images
    pub only_new: bool,
}

pub async fn reindex_images<'a, E: sqlx::Executor<'a, Database = sqlx::Postgres>>(
    executor: E,
    ipc: ImageReindexConfig,
) -> TiberiusResult<()> {
    run_job.builder().set_json(&ipc)?.spawn(executor).await?;
    Ok(())
}

#[instrument(skip(current_job, sctx))]
#[sqlxmq::job]
pub async fn run_job(current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    sentry::configure_scope(|scope| {
        scope.clear();
    });
    let tx = sentry::start_transaction(sentry::TransactionContext::new(
        "reindex_images",
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
    debug!("Job {}: Reindexing images", current_job.id());
    let start = std::time::Instant::now();
    let pool = current_job.pool();
    let progress: ImageReindexConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    debug!(
        "Job {}: Reindexing listed images ({:?})",
        current_job.id(),
        progress.image_ids
    );
    let mut client = sctx.client;
    debug!(
        "Job {}: Completed creating missing metadata rows",
        current_job.id()
    );
    match progress.image_ids {
        None if !progress.only_new => reindex_all(pool, &mut client).await?,
        None if progress.only_new => {
            reindex_new(&mut client).await?;
        }
        Some(v) if !progress.only_new => reindex_many(&mut client, v).await?,
        Some(v) if progress.only_new => {
            reindex_many(&mut client, v).await?;
            reindex_new(&mut client).await?;
        }
        _ => unreachable!(),
    }
    debug!("Job {}: Reindex complete!", current_job.id());
    current_job.complete().await?;
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

#[instrument]
pub async fn reindex_many(client: &mut Client, ids: Vec<i64>) -> TiberiusResult<()> {
    let images = Image::get_many(client, ids, ImageSortBy::Random).await?;
    let index_writer = client.index_writer::<Image>().await?;
    debug!("Reindexing all images, streaming from DB...");
    for image in images {
        debug!("Reindexing image {} {:?}", image.id, image.image);
        image.delete_from_index(index_writer.clone()).await?;
        image.index(index_writer.clone(), client).await?;
    }
    let mut index_writer = index_writer.write().await;
    index_writer.commit()?;
    drop(index_writer);
    Ok(())
}

#[instrument]
pub async fn reindex_new(client: &mut Client) -> TiberiusResult<()> {
    let i = client.index_reader::<Image>()?;
    let dir = ImageSortBy::CreatedAt(tiberius_models::SortDirection::Descending);
    let (_, last_image) = Image::search_item(
        &i,
        tiberius_search::Query::True,
        Vec::new(),
        Vec::new(),
        1,
        0,
        dir,
    )?;
    if last_image.is_empty() {
        warn!("Reindex new failed, no images in index?");
        return Ok(());
    }
    let last_db_image = Image::get_newest(client)
        .await?
        .expect("this job requires atleast one image in the database");
    debug!(
        "Latest indexed image is {}, latest image in database is {}",
        last_image[0].1, last_db_image.id
    );
    if last_image[0].1 == last_db_image.id as u64 {
        debug!("No new images, reindex job step complete");
    } else {
        let images = Image::get_range(client, (last_image[0].1)..(last_db_image.id as u64))
            .await?
            .iter()
            .map(|x| x.id as i64)
            .collect_vec();
        reindex_many(client, images).await?;
        debug!("New images have been indexed");
    }
    Ok(())
}

#[instrument]
pub async fn reindex_all(pool: &Pool<Postgres>, client: &mut Client) -> TiberiusResult<()> {
    let image_count = Image::count(&mut Client::new(pool.clone(), None), None, None).await?;
    let mut images = Image::get_all(pool.clone(), None, None).await?;
    let index_writer = client.index_writer::<Image>().await?;
    let index_reader = client.index_reader::<Image>()?;
    debug!("Reindexing {image_count} images, streaming from DB...");
    let mut counter = 0;
    let progress_indicator = image_count / 521;
    while let Some(image) = images.next().await.transpose()? {
        if counter % progress_indicator == 0 {
            debug!(
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
                    debug!("Reindexing image {counter}/{image_count}: {}", image.id);
                } else {
                    debug!(
                        "Image {counter}/{image_count} requires no reindex: {}",
                        image.id
                    );
                    continue;
                }
            }
            None => {
                debug!(
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
