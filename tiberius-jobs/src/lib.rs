//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(deprecated)]

pub mod cleanup_sessions;
pub mod generate_thumbnails;
#[cfg(feature = "job_process_image")]
pub mod process_image;
pub mod refresh_cachelines;
pub mod refresh_channels;
pub mod reindex_images;
pub mod reindex_tags;
pub mod scheduler;

use std::error::Error;
use std::str::FromStr;

use scheduler::Scheduler;
use tiberius_core::NodeId;
use tiberius_core::{
    app::DBPool, config::Configuration, error::TiberiusResult, state::TiberiusState,
};
use tiberius_dependencies::chrono::Duration;
use tiberius_dependencies::chrono::Utc;
use tiberius_dependencies::cron::Schedule;
use tiberius_dependencies::{prelude::*, serde_json};
use tiberius_dependencies::tokio;
use tiberius_models::Client;

use crate::scheduler::{Instant, Job, CurrentJob};

#[derive(Clone, Debug)]
pub struct SharedCtx {
    client: Client,
    config: Configuration,
}

impl SharedCtx {
    pub fn new(client: Client, config: Configuration) -> Self {
        Self { client, config, }
    }
    pub fn client(&self) -> Client {
        self.client.clone()
    }
    pub fn config(&self) -> Configuration {
        self.config.clone()
    }
}

pub fn job_err_handler(name: &str, err: Box<dyn Error + Send + 'static>) {
    error!("Job {} failed with {:?} ({:?}) ", name, err, err.source());
}

pub async fn scheduler(db: DBPool, config: Configuration) -> ! {
    info!("Booting scheduler");
    let mut sched = scheduler::Scheduler::new(config.node_id(), SharedCtx{
        client: Client::new(db, config.search_dir.as_ref()),
        config: config.clone(),
    });

    {
        info!("Setting up Livestreaming Refresh Job");
        sched.add(Job {
            interval: Some(Schedule::from_str("0 0,30 * * * * *").unwrap()),
            max_delay: Duration::seconds(10),
            last: Utc::now(),
            fun: Box::new(move |i: Instant, current_job: CurrentJob, sctx: SharedCtx| -> TiberiusResult<()> {
                info!("Starting picarto_tv job on scheduler instant {:?}", i);
                let config = refresh_channels::PicartoConfig::default();
                tokio::spawn(async move {
                    refresh_channels::run_job(current_job, sctx).await?;
                    TiberiusResult::<()>::Ok(())
                });
                Ok(())
            }),
        });
    }
    {
        info!("Setting up Image Reindex Job");
        sched.add(Job {
            interval: Some(Schedule::from_str("0 * * * * * *").unwrap()),
            max_delay: Duration::seconds(10),
            last: Utc::now(),
            fun: Box::new(move |i: Instant, current_job: CurrentJob, sctx: SharedCtx| -> TiberiusResult<()> {
                info!("Starting reindex_images job");
                let config = reindex_images::ImageReindexConfig {
                    only_new: true,
                    ..Default::default()
                };
                tokio::spawn(async move {
                    reindex_images::run_job(current_job, sctx).await?;
                    TiberiusResult::<()>::Ok(())
                });
                Ok(())
            }),
        });
    }
    {
        info!("Setting up Tag Reindex Job");
        sched.add(Job {
            interval: Some(Schedule::from_str("0 0 * * * * *").unwrap()),
            max_delay: Duration::seconds(10),
            last: Utc::now(),
            fun: Box::new(move |i: Instant, current_job: CurrentJob, sctx: SharedCtx| -> TiberiusResult<()> {
                info!("Starting reindex_tags job");
                let config = reindex_tags::TagReindexConfig::default();
                tokio::spawn(async move {
                    reindex_tags::run_job(current_job, sctx).await?;
                    TiberiusResult::<()>::Ok(())
                });
                Ok(())
            }),
        });
    }
    {
        info!("Setting up Session Cleanup Job");
        sched.add(Job {
            interval: Some(Schedule::from_str("0 1/10 * * * * *").unwrap()),
            max_delay: Duration::seconds(10),
            last: Utc::now(),
            fun: Box::new(move |i: Instant, current_job: CurrentJob, sctx: SharedCtx| -> TiberiusResult<()> {
                info!("Starting cleanup_sessions job");
                tokio::spawn(async move {
                    cleanup_sessions::run_job(current_job, sctx).await?;
                    TiberiusResult::<()>::Ok(())
                });
                Ok(())
            }),
        });
    }

    info!("Starting scheduler");
    loop {
        let time_to_next = sched.time_to_next();
        if time_to_next.num_milliseconds() < 0 {
            warn!(
                "Scheduler is being fucky, sleeping and forcing an update of the scheduler state"
            );
            sched.force_update_next_tick();
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }
        info!(
            "Next scheduler tick: {:.3} sec",
            time_to_next.num_milliseconds() as f64 / 1000f64
        );
        tokio::time::sleep(time_to_next.to_std().unwrap()).await;
        match sched.run_unticked_jobs::<tiberius_core::error::TiberiusError>(Utc::now()) {
            Ok(_) => (),
            Err(e) => {
                error!("Error in scheduler: {e:?}");
            }
        };
        tokio::task::yield_now().await;
    }
    panic!("Returned from scheduler")
}
