//TODO: fix all these warnings once things settle
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_code)]

#[cfg(all(feature = "stable-release", feature = "full-release"))]
compile_error!("Cannot enable \"stable-release\" and \"full-release\" features at the same time");

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate tracing;

use std::{path::Path, str::FromStr};

use clap::{AppSettings, StructOpt};
use tracing::{debug, info};

use rocket::yansi::Paint;
use sqlx::Postgres;
use tiberius_core::app::DBPool;
use tiberius_core::config::Configuration;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::session::PostgresSessionStore;
use tiberius_core::state::TiberiusState;
use tiberius_core::{package_full, package_name, package_version, CSPHeader};

mod api;
mod cli;
mod init;
mod pages;
#[cfg(test)]
mod tests;

const MAX_IMAGE_DIMENSION: u32 = 2_000_000u32;

fn main() -> TiberiusResult<()> {
    crate::init::logging();
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
        .enable_all()
        .build()
        .unwrap();
    let app = cli::AppCli::parse();
    let app = todo!();/*
    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("server") {
        info!("Starting {}", package_full());
        let job_runner = !matches.is_present("no-jobs");
        if !job_runner {
            warn!("Running without job scheduler and job runner");
        }
        runtime.block_on(async move {
            tokio::spawn(async move {
                crate::cli::server::server_start(job_runner).await
            })
            .await
        })??;
        runtime.shutdown_timeout(std::time::Duration::from_secs(10));
        Ok(())
    } else if let Some(matches) = matches.subcommand_matches("verify-db") {
        #[cfg(feature = "verify-db")]
        runtime.block_on(async move {
            crate::cli::verify_db::verify_db(matches).await
        })?;
        Ok(())
    } else if let Some(matches) = matches.subcommand_matches("grant-acl") {
        runtime.block_on(async move {
            crate::cli::grant_acl::grant_acl(matches).await
        })?;
        Ok(())
    } else if let Some(matches) = matches.subcommand_matches("gen-keys") {
        let base_path = matches
            .value_of("key-directory")
            .expect("must have key directory");
        let base_path = std::path::PathBuf::from_str(base_path)?;
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
        Ok(())
    } else {
        error!("No subcommand specified, please tell me what to do or use --help");
        Ok(())
    }*/
    Ok(())
}
