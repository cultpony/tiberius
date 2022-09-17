//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]

#[cfg(all(feature = "stable-release", feature = "full-release"))]
compile_error!("Cannot enable \"stable-release\" and \"full-release\" features at the same time");

#[macro_use]
extern crate tracing;

use std::{path::Path, str::FromStr};

use clap::{AppSettings, StructOpt};
use tiberius_dependencies::sentry;
use tracing::{debug, info};

use sqlx::Postgres;
use tiberius_core::{
    app::DBPool,
    config::Configuration,
    error::{TiberiusError, TiberiusResult},
    package_full, package_name, package_version,
    session::PostgresSessionStore,
    state::TiberiusState,
    CSPHeader,
};

mod api;
mod cli;
mod init;
mod pages;
#[cfg(test)]
mod tests;

const MAX_IMAGE_DIMENSION: u32 = 2_000_000u32;

#[macro_export]
macro_rules! set_scope_tx {
    ($scope_ident:expr) => {
        tiberius_dependencies::sentry::configure_scope(|scope| {
            scope.set_transaction(Some($scope_ident))
        });
    };
}

#[macro_export]
macro_rules! set_scope_user {
    ($scope_user:expr) => {
        tiberius_dependencies::sentry::configure_scope(|scope| {
            scope.set_user($scope_user);
        })
    };
}

fn main() -> TiberiusResult<()> {
    if let Err(e) = kankyo::load(false) {
        info!("couldn't load .env file: {}, this is probably fine", e);
    }
    let app = cli::AppCli::parse();
    crate::init::logging(&app.config);
    use tokio::runtime::Builder;
    let runtime = Builder::new_multi_thread()
        .worker_threads(16)
        .max_blocking_threads(16)
        .thread_name_fn(|| {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("tiberius-{}", id)
        })
        .thread_stack_size(64 * 1024 * 1024)
        .enable_all()
        .build()
        .unwrap();
    let guard_url = app.config.sentry_url.clone();
    let guard = match guard_url {
        Some(guard_url) => {
            let opts = sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 1.0,
                auto_session_tracking: false,
                session_mode: sentry::SessionMode::Request,
                ..Default::default()
            };
            info!("Starting with sentry tracing");
            Some(sentry::init((guard_url, opts)))
        }
        None => {
            info!("Starting without tracing");
            None
        }
    };
    use cli::Command;
    let global_config = app.config.clone();
    match app.command {
        Command::Server(config) => {
            info!("Starting {}", package_full());
            let job_runner = !config.no_jobs;
            if !job_runner {
                warn!("Running without job scheduler and job runner");
            }
            let scheduler = !config.no_scheduler;
            if !scheduler {
                warn!("Running without job scheduler, worker only mode");
            }
            runtime.block_on(async move {
                tokio::spawn(async move {
                    crate::cli::server::server_start(scheduler, job_runner, global_config).await
                })
                .await
            })??;
            runtime.shutdown_timeout(std::time::Duration::from_secs(10));
            drop(guard);
            Ok(())
        }
        Command::Worker(_) => {
            info!("Starting {} worker", package_full());
            runtime.block_on(async move {
                tokio::spawn(async move {
                    crate::cli::worker::worker_start(global_config).await
                })
                .await
            })??;
            runtime.shutdown_timeout(std::time::Duration::from_secs(10));
            drop(guard);
            Ok(())
        }
        Command::GenKeys(config) => {
            let base_path = std::path::PathBuf::from_str(&config.key_directory)?;
            if !base_path.exists() {
                info!("Creating keys directory...");
                std::fs::create_dir_all(&base_path)?;
            }
            let rng = ring::rand::SystemRandom::new();
            info!("Generting keys...");
            let ed25519path = base_path.join(Path::new("ed25519.pkcs8"));
            let mainkeypath = base_path.join(Path::new("main.key"));

            let sessionkeypath = base_path.join(Path::new("session.key"));
            if !ed25519path.exists() {
                info!("Generating signing key");
                let signing_key = ring::signature::Ed25519KeyPair::generate_pkcs8(&rng)?;
                std::fs::write(ed25519path, signing_key.as_ref())?;
            }
            if !mainkeypath.exists() {
                info!("Generating main key");
                let random_key: [u8; 64] = ring::rand::generate(&rng)?.expose();
                //TODO: generate other needed keys
                std::fs::write(mainkeypath, random_key.as_ref())?;
            }
            if !sessionkeypath.exists() {
                info!("Generating session key");
                let random_key: [u8; 64] = ring::rand::generate(&rng)?.expose();
                std::fs::write(sessionkeypath, random_key.as_ref())?;
            }
            warn!("Keys generated, you are ready to roll.");
            error!("MAKE BACKUPS OF THE {} DIRECTORY", base_path.display());
            drop(guard);
            Ok(())
        }
        Command::GrantAcl(config) => {
            runtime.block_on(async move { crate::cli::grant_acl::grant_acl(&config, global_config).await })?;
            drop(guard);
            Ok(())
        }
        Command::ListUsers(config) => {
            drop(guard);
            todo!()
        }
        Command::RunJob(runjob) => {
            runtime.block_on(async move {
                crate::cli::run_job::run_job(runjob, global_config).await
            })?;
            drop(guard);
            Ok(())
        }
        Command::ExecJob(runjob) => {
            runtime.block_on(async move {
                crate::cli::run_job::exec_job(runjob, global_config).await
            })?;
            drop(guard);
            Ok(())
        }
    }
}
