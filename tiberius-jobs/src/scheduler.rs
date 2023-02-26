use std::collections::HashMap;

use tiberius_dependencies::chrono::{DateTime, Utc, Duration};
use futures_util::future::BoxFuture;
use sqlx::Postgres;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::NodeId;
use tiberius_dependencies::{cron, atomic, prelude::*, uuid::Uuid};
use atomic::{Atomic, Ordering};

#[derive(Debug)]
pub struct Scheduler {
    jobs: HashMap<Uuid, Job>,
    next_up: Vec<Uuid>,
    next_scheduled: Atomic<DateTime<Utc>>,
    node_id: NodeId,
    shortest_delay: Duration,
}

impl Scheduler {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            jobs: HashMap::new(),
            next_up: Vec::new(),
            next_scheduled: Atomic::new(DateTime::<Utc>::MAX_UTC),
            shortest_delay: Duration::days(1),
            node_id,
        }
    }
    pub fn add(&mut self, j: Job) {
        self.jobs.insert(self.node_id.uuid(), j);
        self.force_update_next_tick();
    }

    /// Updates the next_scheduled variable in the struct to the nearest datetime that must
    /// be scheduled.
    /// 
    /// Will only update if the next_scheduled datetime has passed
    fn update_next_tick(&mut self) -> bool {
        if self.next_scheduled.load(Ordering::SeqCst) > Utc::now() {
            false
        } else {
            self.force_update_next_tick()
        }
    }

    pub fn force_update_next_tick(&mut self) -> bool {
        let mut next = DateTime::<Utc>::MAX_UTC;
        let mut shortest = Duration::days(1);
        let now = Utc::now();
        for (_, job) in &self.jobs {
            match job.interval.after(&now).next() {
                None => (),
                Some(time) if time < next => {
                    debug!("Found better schedule: {time:?} over {next:?}");
                    next = time
                },
                Some(_) => (),
            }
            if job.max_delay < shortest {
                shortest = job.max_delay;
            }
        }
        debug!("New next schedule is {next:?}");
        self.shortest_delay = shortest;
        self.next_scheduled.store(next, Ordering::SeqCst);
        true
    }

    pub fn time_to_next(&self) -> tiberius_dependencies::chrono::Duration {
        self.next_scheduled.load(Ordering::SeqCst) - Utc::now()
    }

    fn unticked_jobs<'a>(&'a mut self, time: DateTime<Utc>) -> (Vec<&'a mut Job>, DateTime<Utc>) {
        let moment = Utc::now();
        let mut jobs = Vec::new();
        for (u, j) in &mut self.jobs {
            let next = j.next(j.last);
            match next {
                None => (),
                Some(next) => {
                    if time > next {
                        jobs.push(j) 
                    }
                }
            }
        }
        (jobs, moment)
    }

    pub fn run_unticked_jobs<E>(&mut self, time: DateTime<Utc>) -> Result<(), TiberiusError> {
        let (jobs, moment) = self.unticked_jobs(time);
        for job in jobs {
            let instant = Instant{
                call_time: Utc::now(),
                sched_time: time,
                plan_time: moment,
            };
            (job.fun).call(instant)?;
            (*job).last = Utc::now();
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.force_update_next_tick();
        Ok(())
    }
}
pub struct Job {
    /// Schedule of the job
    pub interval: cron::Schedule,
    /// Record the last time the job ran
    /// 
    /// When creating, this should be set to Utc::now, otherwise it's usable
    /// to determine when the job will *first* run.
    pub last: tiberius_dependencies::chrono::DateTime<Utc>,
    /// Maximum allowed time that scheduling may be delayed
    ///
    /// The scheduler will check the current datetime as often as the smallest
    /// max_delay value over all jobs unless the next scheduling event is more than
    /// 10 times as far away.
    pub max_delay: tiberius_dependencies::chrono::Duration,
    /// The function call that will run the job
    ///
    /// If it errors, the job is marked as failed until it completes normally again
    pub fun: Box<dyn JobCallable<Err = TiberiusError> + Send>,
}

impl Job {
    pub fn next(&self, time: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.interval.after(&time).next()
    }
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("interval", &self.interval)
            .field("max_delay", &self.max_delay)
            .field("fun", &"Fn()")
            .finish()
    }
}

#[derive(Debug)]
pub struct Instant {
    /// The time the task was called
    pub call_time: DateTime<Utc>,
    /// The time that scheduling was considered
    pub sched_time: DateTime<Utc>,
    /// The time that the task was planned to start
    pub plan_time: DateTime<Utc>,
}

pub trait JobCallable {
    type Err;

    fn call(&self, i: Instant) -> Result<(), Self::Err>;
}

impl<F, E> JobCallable for F
where
    F: Fn(Instant) -> Result<(), E> + Send + Sync,
{
    type Err = E;

    fn call(&self, i: Instant) -> Result<(), Self::Err> {
        self(i)
    }
}
