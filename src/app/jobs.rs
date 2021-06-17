mod cleanup_sessions;
mod picarto_tv;

use std::error::Error;

use log::{error, info};
use sqlxmq::JobRegistry;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{app::{jobs::picarto_tv::PicartoConfig, DBPool}, config::Configuration, error::TiberiusResult};

pub async fn runner(db: DBPool) -> TiberiusResult<sqlxmq::OwnedHandle> {
    let mut registry = JobRegistry::new(&[picarto_tv::run_job, cleanup_sessions::run_job]);
    registry.set_error_handler(job_err_handler);
    Ok(registry.runner(&db).set_concurrency(10, 20).run().await?)
}

pub fn job_err_handler(name: &str, err: Box<dyn Error + Send + 'static>) {
    error!("Job {} failed with {:?}", name, err);
}

pub async fn scheduler(db: DBPool, config: Configuration) -> ! {
    let mut sched = JobScheduler::new();

    {
        let db = db.clone();
        let config = config.clone();
        sched
            .add(
                Job::new("0 1/10 * * * * *", move |uuid, l| {
                    info!("Starting picarto_tv job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    let config = config.clone();
                    let config = PicartoConfig {
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
                .unwrap(),
            )
            .unwrap();
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
                .unwrap(),
            )
            .unwrap();
    }

    sched.start().await.expect("scheduler failed");
    error!("scheduler exited");
    drop(sched);
    panic!("returned from scheduler");
}
