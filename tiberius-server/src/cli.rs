use clap::{ArgEnum, Args, Parser, Subcommand};

pub mod grant_acl;
pub mod list_users;
pub mod run_job;
pub mod server;

#[derive(Parser, Debug)]
#[clap(author, version, about = "The Lunar Image Board", long_about = None)]
pub struct AppCli {
    #[clap(subcommand)]
    pub command: Command,
    #[clap(flatten)]
    pub config: tiberius_core::config::Configuration,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Launch the Image Board to the Moon!
    Server(ServerCli),
    #[cfg(feature = "verify-db")]
    /// Verifies the integrity of the Database by reading all rows and checking their relationships manually
    /// This command exists to counter database corruption. It is recommended to run this after a restore
    /// as it will flag missing files or database entries.
    VerifyDb(VerifyDbCli),
    /// Generate Security Keys and Data for Tiberius to run
    /// You should only run this once, this command will abort if it finds existing key material on disk.
    GenKeys(GenKeysCli),
    /// Search Database for users
    ListUsers(ListUsersCli),
    /// Basic Access Management for Tiberius, to be used to promote users to admin if access to existing admin accounts is lost
    /// or during bootstrapping your installation.
    GrantAcl(GrantAclCli),
    /// Run a specific job manually. Note that you will only schedule the job, a worker must be available
    RunJob(RunJobCli),
}

#[derive(Args, Debug)]
pub struct ServerCli {
    #[clap(long, short = 'z', alias = "no-jobrunner")]
    /// Disable all job processing, including scheduler
    pub no_jobs: bool,
    #[clap(long, short = 'y')]
    /// Disable the scheduler, only run a worker
    pub no_scheduler: bool,
}

#[derive(Args, Debug)]
pub struct GenKeysCli {
    pub key_directory: String,
}

#[derive(Args, Debug)]
pub struct ListUsersCli {
    /// Test to search in user database table, must be 5 characters or more
    #[clap(value_name = "TERM", validator = validate_search)]
    pub search: String,
}

fn validate_search(x: &str) -> Result<(), String> {
    if x.len() > 5 {
        Ok(())
    } else {
        Err("Search term must be 5 characters or more".to_string())
    }
}

#[derive(Args, Debug)]
pub struct GrantAclCli {
    #[clap(subcommand)]
    pub act: GrantAclAction,
    #[clap(long)]
    pub user: Option<String>,
    #[clap(long)]
    pub group: Option<String>,
    #[clap(long)]
    pub member_of: Option<String>,
    #[clap(long)]
    pub subject: Option<String>,
    #[clap(long)]
    pub action: Option<String>,
}

#[derive(Subcommand, Debug, PartialEq, Eq, Copy, Clone)]
pub enum GrantAclAction {
    Grant,
    Revoke,
    List,
}

#[derive(Args, Debug)]
pub struct RunJobCli {
    #[clap(subcommand)]
    pub job: RunJobSelect,
}

#[derive(Subcommand, Debug)]
pub enum RunJobSelect {
    RefreshCachelines {
        image_start: u64,
        #[clap(requires("image-start"))]
        image_end: Option<u64>,
    },
}
