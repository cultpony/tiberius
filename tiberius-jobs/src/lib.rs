//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(deprecated)]

pub mod cleanup_sessions;
#[cfg(feature = "job_process_image")]
pub mod process_image;
pub mod refresh_cachelines;
pub mod refresh_channels;
pub mod reindex_images;
pub mod reindex_tags;
pub mod generate_thumbnails;
pub mod scheduler;

use std::error::Error;
use std::str::FromStr;

use chrono::Duration;
use sqlxmq::JobRegistry;
use tiberius_core::{
    app::DBPool, config::Configuration, error::TiberiusResult, state::TiberiusState,
};
use tiberius_dependencies::cron::Schedule;
use tiberius_models::Client;
use tiberius_dependencies::prelude::*;
use chrono::Utc;

use crate::scheduler::{Instant, Job};

#[derive(Clone, Debug)]
pub struct SharedCtx {
    client: Client,
    config: Configuration,
}

pub fn registry() -> TiberiusResult<JobRegistry> {
    Ok(JobRegistry::new(&[
        refresh_channels::run_job,
        cleanup_sessions::run_job,
        reindex_images::run_job,
        reindex_tags::run_job,
        #[cfg(feature = "job_process_image")]
        process_image::run_job,
        refresh_cachelines::run_job,
        generate_thumbnails::run_job,
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
    info!("Booting scheduler");
    let mut sched = scheduler::Scheduler::new(config.node_id());
    {
        info!("Setting up Livestreaming Refresh Job");
        let db = db.clone();
        sched.add(Job {
            interval: Schedule::from_str("0 0,30 * * * * *").unwrap(),
            max_delay: Duration::seconds(10),
            last: Utc::now(),
            fun: Box::new(move |i: Instant| -> TiberiusResult<()> {
                info!("Starting picarto_tv job on scheduler instant {:?}", i);
                let db = db.clone();
                let config = refresh_channels::PicartoConfig::default();
                tokio::spawn(async move {
                    let mut jb: sqlxmq::JobBuilder = refresh_channels::run_job.builder();
                    jb.set_json(&config)
                        .expect("could not serialize job config")
                        .spawn(&db)
                        .await
                });
                Ok(())
            })
        });
    }
    {
        info!("Setting up Image Reindex Job");
        let db = db.clone();
        sched.add(Job {
            interval: Schedule::from_str("0 * * * * * *").unwrap(),
            max_delay: Duration::seconds(10),
            last: Utc::now(),
            fun: Box::new(move |i: Instant| -> TiberiusResult<()> {
                info!("Starting reindex_images job");
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
                Ok(())
            }),
        })
    }
    {
        info!("Setting up Tag Reindex Job");
        let db = db.clone();
        sched.add(Job {
            interval: Schedule::from_str("0 0 * * * * *").unwrap(),
            max_delay: Duration::seconds(10),
            last: Utc::now(),
            fun: Box::new(move |i: Instant| -> TiberiusResult<()> {
                info!("Starting reindex_tags job");
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
                Ok(())
            }),
        })
    }
    {
        info!("Setting up Session Cleanup Job");
        let db = db.clone();
        sched.add(Job {
            interval: Schedule::from_str("0 1/10 * * * * *").unwrap(),
            max_delay: Duration::seconds(10),
            last: Utc::now(),
            fun: Box::new(move |i: Instant| -> TiberiusResult<()> {
                info!("Starting cleanup_sessions job");
                let db = db.clone();
                tokio::spawn(async move {
                    let jb: sqlxmq::JobBuilder = cleanup_sessions::run_job.builder();
                    jb.spawn(&db).await
                });
                Ok(())
            }),
        })
    }
    
    info!("Starting scheduler");
    loop {
        let time_to_next = sched.time_to_next();
        if time_to_next.num_milliseconds() < 0 {
            warn!("Scheduler is being fucky, sleeping and forcing an update of the scheduler state");
            sched.force_update_next_tick();
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }
        info!("Next scheduler tick: {:.3} sec", time_to_next.num_milliseconds() as f64 / 1000f64);
        tokio::time::sleep(time_to_next.to_std().unwrap()).await;
        match sched.run_unticked_jobs::<tiberius_core::error::TiberiusError>(Utc::now()) {
            Ok(_) => (),
            Err(e) => {
                error!("Error in scheduler: {e:?}");
                ()
            }
        };
        tokio::task::yield_now().await;
    }
    panic!("Returned from scheduler")
}
