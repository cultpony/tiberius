use crate::cli::{RunJobCli, ExecJobCli};
use tiberius_core::{app::DBPool, config::Configuration, error::TiberiusResult};
use tiberius_jobs::{refresh_cachelines, reindex_images};
use tiberius_models::Client;

pub async fn run_job(run_job: RunJobCli, config: Configuration) -> TiberiusResult<()> {
    info!("Starting with config {:?}", config);
    let db_conn: DBPool = config.db_conn().await?;
    match run_job.job {
        crate::cli::RunJobSelect::RefreshCachelines {
            image_start,
            image_end,
        } => {
            let config = refresh_cachelines::RefreshCachelineConfig {
                image_id_range: (image_start)..(image_end.unwrap_or(u64::MAX)),
            };
            let mut jb: sqlxmq::JobBuilder = refresh_cachelines::run_job.builder();
            jb.set_json(&config)
                .expect("could not serialize job config")
                .spawn(&db_conn)
                .await?;
        },
        crate::cli::RunJobSelect::ReindexImages { only_new } => {
            let config = reindex_images::ImageReindexConfig{
                only_new,
                ..Default::default()
            };
            let mut jb: sqlxmq::JobBuilder = reindex_images::run_job.builder();
            jb.set_json(&config)
                .expect("could not serialize job config")
                .spawn(&db_conn)
                .await?;
        }
    }
    info!("Tiberius exited.");
    Ok(())
}

pub async fn exec_job(run_job: ExecJobCli, config: Configuration) -> TiberiusResult<()> {
    info!("Starting with config {:?}", config);
    let db_conn: DBPool = config.db_conn().await?;
    let mut client = Client::new(db_conn, config.search_dir.as_ref());
    match run_job.job {
        crate::cli::ExecJobSelect::ReindexNewImages => {
            reindex_images::reindex_new(&mut client).await?;
        }
    }
    info!("Tiberius exited.");
    Ok(())
}
