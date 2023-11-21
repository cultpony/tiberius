use std::collections::HashMap;
use std::ops::Sub;
use std::sync::{RwLock, Arc, Mutex};
use std::fmt::Debug;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::NodeId;
use tiberius_dependencies::atomic::{Atomic, Ordering};
use tiberius_dependencies::chrono::{DateTime, Duration, Utc};
use tiberius_dependencies::futures_util::future::BoxFuture;
use tiberius_dependencies::{serde, serde_json};
use tiberius_dependencies::sqlx::Postgres;
use tiberius_dependencies::{atomic, cron, prelude::*, uuid::Uuid};

#[derive(Debug)]
pub struct Scheduler<SharedCtx: Send + Sync + Clone + Debug> {
    jobs: RwLock<HashMap<Uuid, JobRef<SharedCtx>>>,
    next_up: Mutex<Vec<Uuid>>,
    next_scheduled: RwLock<Box<DateTime<Utc>>>,
    node_id: NodeId,
    shortest_delay: RwLock<Duration>,
    context: SharedCtx,
}

unsafe impl<S: Send + Sync + Clone + Debug> Sync for Scheduler<S> {}

pub type JobRef<SharedCtx> = std::sync::Arc<std::sync::Mutex<Job<SharedCtx>>>;

pub struct CurrentJob{
    uuid: Uuid,
    data: Option<serde_json::Value>,
}

impl CurrentJob {
    pub fn new<S: Send + Sync + Clone + Debug>(sched: &Scheduler<S>) -> Self {
        Self{
            uuid: sched.node_id.uuid(),
            data: None,
        }
    }
    pub fn with_data<T: serde::Serialize + serde::de::DeserializeOwned>(mut self, data: T) -> TiberiusResult<Self> {
        self.data = Some(serde_json::to_value(data)?);
        Ok(self)
    }
    pub fn data<T: serde::de::DeserializeOwned>(&self) -> TiberiusResult<Option<T>> {
        match self.data.as_ref() {
            Some(data) => Ok(serde_json::from_value(data.clone())?),
            None => Ok(None)
        }
    }
    pub fn id(&self) -> Uuid {
        self.uuid
    }
}

impl Default for CurrentJob {
    fn default() -> Self {
        Self{ uuid: NodeId::default().uuid(), data: None }
    }
}


impl<S: Send + Sync + Clone + Debug> Scheduler<S> {
    pub fn new(node_id: NodeId, context: S) -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
            next_up: Mutex::new(Vec::new()),
            next_scheduled: RwLock::new(Box::new(DateTime::<Utc>::MAX_UTC)),
            shortest_delay: RwLock::new(Duration::days(1)),
            node_id,
            context,
        }
    }
    fn new_current_job(&self) -> CurrentJob {
        CurrentJob::new(&self)
    }
    pub fn add(&mut self, j: Job<S>) -> Uuid {
        let uuid = self.node_id.uuid();
        self.jobs.write().unwrap().insert(uuid, Arc::new(Mutex::new(j)));
        self.force_update_next_tick();
        uuid
    }

    pub fn add_immediate(&mut self, j: Job<S>) -> TiberiusResult<Uuid> {
        if let Some(schedule) = j.interval {
            return Err(TiberiusError::ImmediateJobSchedule(None, schedule))
        }
        let uuid = self.node_id.uuid();
        self.jobs.write().unwrap().insert(uuid, Arc::new(Mutex::new(j)));
        self.next_up.lock().unwrap().push(uuid);
        self.force_update_next_tick();
        Ok(uuid)
    }

    /// Runs the job and removes it from the schedule
    /// 
    /// Panics if a job is immediated and it has a schedule
    pub fn immediate_schedule(&mut self, uuid: Uuid) -> TiberiusResult<()> {
        let jobs = self.jobs.read().unwrap();
        if let Some(j) = jobs.get(&uuid) {
            if let Some(schedule) = j.lock().unwrap().interval.clone() {
                return Err(TiberiusError::ImmediateJobSchedule(Some(uuid), schedule))
            }
            self.next_up.lock().unwrap().push(uuid);
            Ok(())
        } else {
            Err(TiberiusError::InvalidJobId(uuid))
        }
    }

    /// Updates the next_scheduled variable in the struct to the nearest datetime that must
    /// be scheduled.
    ///
    /// Will only update if the next_scheduled datetime has passed
    fn update_next_tick(&mut self) -> bool {
        if **self.next_scheduled.read().unwrap() > Utc::now() {
            false
        } else {
            self.force_update_next_tick()
        }
    }

    pub fn force_update_next_tick(&self) -> bool {
        let mut next = DateTime::<Utc>::MAX_UTC;
        let mut shortest = Duration::days(1);
        let now = Utc::now();
        let jobs = self.jobs.read().unwrap();
        for job in jobs.values() {
            match job.lock().unwrap().interval.clone().map(|x| x.after(&now).next()).flatten() {
                None => (),
                Some(time) if time < next => {
                    debug!("Found better schedule: {time:?} over {next:?}");
                    next = time
                }
                Some(_) => (),
            }
            let max_delay = job.lock().unwrap().max_delay;
            if max_delay < shortest {
                shortest = max_delay;
            }
        }
        debug!("New next schedule is {next:?}");
        *(self.shortest_delay.write().unwrap()) = shortest;
        let mut wns = self.next_scheduled.write().unwrap();
        **wns = next;
        true
    }

    pub fn time_to_next(&self) -> tiberius_dependencies::chrono::Duration {
        self.next_scheduled.read().unwrap().sub(Utc::now())
    }

    fn unticked_jobs(&self, time: DateTime<Utc>) -> (Vec<JobRef<S>>, DateTime<Utc>) {
        let moment = Utc::now();
        let mut jobs = Vec::new();
        let jobsg = self.jobs.read().unwrap();
        for (u, j) in jobsg.iter() {
            let jg = j.lock().unwrap();
            let next = jg.next(jg.last, self.context.clone());
            match next {
                None => (),
                Some(next) => {
                    if time > next {
                        jobs.push(j.clone())
                    }
                }
            }
        }
        (jobs, moment)
    }

    pub fn run_unticked_jobs<E>(&self, time: DateTime<Utc>) -> Result<(), TiberiusError> {
        let (jobs, moment) = self.unticked_jobs(time);
        for job in jobs {
            let instant = Instant {
                call_time: Utc::now(),
                sched_time: time,
                plan_time: moment,
            };
            let mut jobg = job.lock().unwrap();
            (jobg.fun).call(instant, self.new_current_job(), self.context.clone())?;
            jobg.last = Utc::now();
        }
        let mut jobsg = self.jobs.write().unwrap();
        for job in self.next_up.lock().unwrap().drain(..).map(|x| jobsg.remove(&x)) {
            match job {
                None => warn!("empty job in next_up schedule"),
                Some(job) => {
                    let instant = Instant {
                        call_time: Utc::now(),
                        sched_time: time,
                        plan_time: moment,
                    };
                    let mut jobg = job.lock().unwrap();
                    (jobg.fun).call(instant, self.new_current_job(), self.context.clone())?;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.force_update_next_tick();
        Ok(())
    }
}
pub struct Job<S: Send + Sync + Clone> {
    /// Schedule of the job
    /// If there is no schedule, the job is removed not run unless it's UUID is manually
    /// added or the add_immediate call was used to schedule it's run.
    pub interval: Option<cron::Schedule>,
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
    pub fun: Box<dyn JobCallable<S, Err = TiberiusError> + Send>,
}

impl<S: Send + Sync + Clone + Debug> Job<S> {
    pub fn next(&self, time: DateTime<Utc>, context: S) -> Option<DateTime<Utc>> {
        self.interval.as_ref().map(|x| x.after(&time).next()).flatten()
    }
}

impl<S: Send + Sync + Clone + Debug> std::fmt::Debug for Job<S> {
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

pub trait JobCallable<SharedCtx: Send + Sync + Clone + Debug> {
    type Err;

    fn call(&self, i: Instant, c: CurrentJob, s: SharedCtx) -> Result<(), Self::Err>;
}

impl<F, E, SharedCtx> JobCallable<SharedCtx> for F
where
    F: Fn(Instant, CurrentJob, SharedCtx) -> Result<(), E> + Send + Sync,
    SharedCtx: Send + Sync + Clone + Debug,
    E: std::error::Error,
{
    type Err = E;

    fn call(&self, i: Instant, c: CurrentJob, s: SharedCtx) -> Result<(), Self::Err> {
        self(i, c, s)
    }
}