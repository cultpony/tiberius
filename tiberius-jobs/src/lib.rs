mod cleanup_sessions;
mod picarto_tv;
pub mod reindex_images;
pub mod reindex_tags;

use std::error::Error;

use log::{error, info};
use sqlxmq::JobRegistry;
use tiberius_core::app::DBPool;
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn runner(db: DBPool) -> TiberiusResult<()> {
    let mut registry = JobRegistry::new(&[
        picarto_tv::run_job,
        cleanup_sessions::run_job,
        reindex_images::run_job,
        reindex_tags::run_job,
    ]);
    registry.set_error_handler(job_err_handler);
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

    {
        let db = db.clone();
        let config = config.clone();
        sched
            .add(
                Job::new("0 0/10 * * * * *", move |uuid, l| {
                    info!("Starting picarto_tv job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    let config = config.clone();
                    let config = picarto_tv::PicartoConfig {
                        config,
                        ..Default::default()
                    };
                    tokio::spawn(async move {
                        let mut jb: sqlxmq::JobBuilder = picarto_tv::run_job.builder();
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
    {
        let db = db.clone();
        sched
            .add(
                Job::new("0 1/20 * * * * *", move |uuid, l| {
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

    {
        let db = db.clone();
        let config = config.clone();
        sched
            .add(
                Job::new("0 * * * * * *", move |uuid, l| {
                    info!("Starting reindex_images job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    let config = config.clone();
                    let config = reindex_images::ImageReindexConfig {
                        config,
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

    {
        let db = db.clone();
        let config = config.clone();
        sched
            .add(
                Job::new("0 * * * * * *", move |uuid, l| {
                    info!("Starting reindex_tags job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    let config = config.clone();
                    let config = reindex_tags::TagReindexConfig {
                        config,
                        ..Default::default()
                    };
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

    sched.start().await.expect("scheduler failed");
    error!("scheduler exited");
    drop(sched);
    panic!("returned from scheduler");
}
