use tiberius_core::{config::Configuration, error::TiberiusResult, state::TiberiusState};
use tiberius_dependencies::futures_util::stream::StreamExt;
use tiberius_dependencies::prelude::*;
use tiberius_dependencies::sentry;
use tiberius_dependencies::serde;
use tiberius_dependencies::serde_json;
use tiberius_dependencies::sqlx::{FromRow, Pool, Postgres};
use tiberius_models::{Channel, Client, Tag, TagLike};

use tiberius_models::Queryable;

use crate::SharedCtx;
use crate::scheduler::CurrentJob;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Default)]
pub struct TagReindexConfig {
    pub tag_ids: Option<Vec<i64>>,
}

#[instrument(skip(current_job, sctx))]
pub async fn run_job(current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    sentry::configure_scope(|scope| {
        scope.clear();
    });
    let tx = sentry::start_transaction(sentry::TransactionContext::new(
        "reindex_tags",
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
async fn tx_run_job(mut current_job: CurrentJob, mut sctx: SharedCtx) -> TiberiusResult<()> {
    debug!("Job {}: Reindexing all tags", current_job.id());
    let start = std::time::Instant::now();
    let mut client = sctx.client.clone();
    let progress: TagReindexConfig = current_job.data()?.unwrap_or_default();
    match progress.tag_ids {
        None => reindex_all(&mut client).await?,
        Some(v) => reindex_many(&mut client, v).await?,
    }
    debug!("Job {}: Reindex complete!", current_job.id());
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

async fn reindex_many(client: &mut Client, ids: Vec<i64>) -> TiberiusResult<()> {
    todo!()
}

#[tracing::instrument]
pub async fn reindex_all(client: &mut Client) -> TiberiusResult<()> {
    let mut tags = Tag::get_all(client, None, None).await?;
    let index_writer = client.index_writer::<Tag>().await?;
    debug!("Reindexing all tags, streaming from DB...");
    while let Some(tag) = tags.next().await.transpose()? {
        let tag: Tag = Tag::from_row(&tag)?;
        trace!("Reindexing tag {}: {}", tag.id, tag.full_name());
        tag.delete_from_index(index_writer.clone()).await?;
        tag.index(index_writer.clone(), client).await?;
    }
    index_writer.write().await.commit()?;
    Ok(())
}
