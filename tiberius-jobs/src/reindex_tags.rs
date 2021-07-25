use log::{info, trace};
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::state::TiberiusState;
use tiberius_models::{Channel, Client, Tag};
use rocket::Request;
use rocket::futures::TryStreamExt;
use sqlx::{FromRow, Pool, Postgres};
use sqlxmq::{job, Checkpoint, CurrentJob};

use tiberius_models::Queryable;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct TagReindexConfig {
    pub config: Configuration,
    pub tag_ids: Option<Vec<i64>>,
}

impl Default for TagReindexConfig {
    fn default() -> Self {
        Self {
            config: Configuration::default(),
            tag_ids: None,
        }
    }
}

#[sqlxmq::job]
pub async fn run_job(mut current_job: CurrentJob) -> TiberiusResult<()> {
    info!("Job {}: Reindexing all tags", current_job.id());
    let pool = current_job.pool();
    let progress: TagReindexConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    let mut client = TiberiusState::get_db_client_standalone(pool.clone(), &progress.config).await?;
    match progress.tag_ids {
        None => reindex_all(pool, &mut client).await?,
        Some(v) => reindex_many(&mut client, v).await?,
    }
    info!("Job {}: Reindex complete!", current_job.id());
    current_job.complete().await?;
    Ok(())
}

async fn reindex_many(client: &mut Client, ids: Vec<i64>) -> TiberiusResult<()> {
    todo!()
}

async fn reindex_all(pool: &Pool<Postgres>, client: &mut Client) -> TiberiusResult<()> {
    let mut tags = Tag::get_all(pool.clone(), None, None).await?;
    let mut index_writer = client.index_writer::<Tag>()?;
    info!("Reindexing all tags, streaming from DB...");
    while let Some(tag) = tags.try_next().await? {
        let tag: Tag = Tag::from_row(&tag)?;
        info!("Reindexing tag {}: {}", tag.id, tag.full_name());
        tag.delete_from_index(&mut index_writer).await?;
        tag.index(&mut index_writer, client).await?;
    }
    index_writer.commit()?;
    Ok(())
}