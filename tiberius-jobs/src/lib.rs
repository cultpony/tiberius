//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(deprecated)]

#[macro_use]
extern crate tracing;

#[cfg(feature = "job_cleanup_sessions")]
pub mod cleanup_sessions;
#[cfg(feature = "job_process_image")]
pub mod process_image;
pub mod refresh_cachelines;
#[cfg(feature = "job_refresh_channels")]
pub mod refresh_channels;
#[cfg(feature = "job_reindex_images")]
pub mod reindex_images;
#[cfg(feature = "job_reindex_tags")]
pub mod reindex_tags;

use std::error::Error;

use sqlxmq::JobRegistry;
use tiberius_core::{
    app::DBPool, config::Configuration, error::TiberiusResult, state::TiberiusState,
};
use tiberius_models::Client;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

#[derive(Clone, Debug)]
pub struct SharedCtx {
    client: Client,
    config: Configuration,
}

pub fn registry() -> TiberiusResult<JobRegistry> {
    Ok(JobRegistry::new(&[
        #[cfg(feature = "job_refresh_channels")]
        refresh_channels::run_job,
        #[cfg(feature = "job_cleanup_sessions")]
        cleanup_sessions::run_job,
        #[cfg(feature = "job_reindex_images")]
        reindex_images::run_job,
        #[cfg(feature = "job_reindex_tags")]
        reindex_tags::run_job,
        #[cfg(feature = "job_process_image")]
        process_image::run_job,
        refresh_cachelines::run_job,
    ]))
}

pub async fn runner(db: DBPool, config: Configuration) -> TiberiusResult<()> {
    let mut registry = registry()?;
    let client = TiberiusState::get_db_client_standalone(db.clone(), &config).await?;
    registry.set_error_handler(job_err_handler);
    registry.set_context(SharedCtx {
        client: client,
        config: config,
    });
    let handle = registry.runner(&db).set_concurrency(1, 20).run().await?;
    let handle = handle.into_inner();
    handle.await?;
    Ok(())
}

pub fn job_err_handler(name: &str, err: Box<dyn Error + Send + 'static>) {
    error!("Job {} failed with {:?} ({:?}) ", name, err, err.source());
}

pub async fn scheduler(db: DBPool, config: Configuration) -> ! {
    let mut sched = JobScheduler::new();

    #[cfg(feature = "job_refresh_channels")]
    {
        let db = db.clone();
        sched
            .add(
                Job::new("0 0/10 * * * * *", move |uuid, l| {
                    info!("Starting picarto_tv job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    let config = refresh_channels::PicartoConfig::default();
                    tokio::spawn(async move {
                        let mut jb: sqlxmq::JobBuilder = refresh_channels::run_job.builder();
                        jb.set_json(&config)
                            .expect("could not serialize job config")
                            .spawn(&db)
                            .await
                    });
                })
                .expect("could not spawn job"),
            )
            .expect("could not add job to scheduler");
    }
    #[cfg(feature = "job_cleanup_sessions")]
    {
        let db = db.clone();
        sched
            .add(
                Job::new("0 1/10 * * * * *", move |uuid, l| {
                    info!("Starting cleanup_sessions job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    tokio::spawn(async move {
                        let jb: sqlxmq::JobBuilder = cleanup_sessions::run_job.builder();
                        jb.spawn(&db).await
                    });
                })
                .expect("could not spawn job"),
            )
            .expect("could not add job to scheduler");
    }
    #[cfg(feature = "job_reindex_images")]
    {
        let db = db.clone();
        sched
            .add(
                Job::new("0 2/10 * * * * *", move |uuid, l| {
                    info!("Starting reindex_images job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    let config = reindex_images::ImageReindexConfig{
                        only_new: true,
                        ..Default::default()
                    };
                    tokio::spawn(async move {
                        let mut jb: sqlxmq::JobBuilder = reindex_images::run_job.builder();
                        let jb = jb.set_json(&config);
                        match jb {
                            Ok(jb) => {
                                jb.spawn(&db).await.expect("could not spawn job");
                            }
                            Err(e) => {
                                error!("could not spawn job: {}", e);
                            }
                        };
                    });
                })
                .expect("could not spawn job"),
            )
            .expect("could not add job to scheduler");
    }
    #[cfg(feature = "job_reindex_tags")]
    {
        let db = db.clone();
        sched
            .add(
                Job::new("0 0 * * * * *", move |uuid, l| {
                    info!("Starting reindex_tags job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    let config = reindex_tags::TagReindexConfig::default();
                    tokio::spawn(async move {
                        let mut jb: sqlxmq::JobBuilder = reindex_tags::run_job.builder();
                        let jb = jb.set_json(&config);
                        match jb {
                            Ok(jb) => {
                                jb.spawn(&db).await.expect("could not spawn job");
                            }
                            Err(e) => {
                                error!("could not spawn job: {}", e);
                            }
                        };
                    });
                })
                .expect("could not spawn job"),
            )
            .expect("could not add job to scheduler");
    }

    info!("Starting scheduler");
    sched.start().await.expect("scheduler failed");
    error!("scheduler exited");
    drop(sched);
    panic!("returned from scheduler");
}
