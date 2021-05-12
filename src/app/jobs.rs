mod picarto_tv;

use std::error::Error;

use anyhow::Result;
use log::{error, info};
use sqlxmq::JobRegistry;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::app::DBPool;

pub async fn runner(db: DBPool) -> Result<sqlxmq::OwnedHandle> {
    let mut registry = JobRegistry::new(&[picarto_tv::run_job]);
    registry.set_error_handler(job_err_handler);
    Ok(registry.runner(&db).set_concurrency(10, 20).run().await?)
}

pub fn job_err_handler(name: &str, err: Box<dyn Error + Send + 'static>) {
    error!("Job {} failed with {:?}", name, err);
}

pub async fn scheduler(db: DBPool) -> ! {
    let mut sched = JobScheduler::new();

    {
        let db = db.clone();
        sched
            .add(
                Job::new("0 1/10 * * * * *", move |uuid, l| {
                    info!("Starting picarto_tv job on scheduler UUID {}", uuid);
                    let db = db.clone();
                    tokio::spawn(async move { picarto_tv::run_job.builder().spawn(&db).await });
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
