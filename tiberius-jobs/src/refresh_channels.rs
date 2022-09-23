pub mod picarto;

use sqlxmq::{job, Checkpoint, CurrentJob};
use tiberius_core::{
    config::Configuration, error::TiberiusResult, http_client, state::TiberiusState,
};
use tiberius_models::{Channel, Client};
use tracing::{debug, info, trace};

use crate::SharedCtx;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PicartoConfig {
    pub config: Configuration,
    pub all_channels: Vec<Channel>,
    pub done_channels: Vec<i32>,
    pub started: bool,
}

impl Default for PicartoConfig {
    fn default() -> Self {
        Self {
            config: Configuration::default(),
            all_channels: Vec::new(),
            done_channels: Vec::new(),
            started: false,
        }
    }
}

#[instrument(skip(current_job, sctx))]
#[sqlxmq::job(retries = 1)]
pub async fn run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    let pool = current_job.pool();
    let progress: PicartoConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    info!("Job {}: Refreshing picarto channels", current_job.id());
    let mut client = sctx.client;
    let mut progress = {
        if progress.started {
            progress
        } else {
            let all_channels =
                Channel::get_all_channels(&mut client, Some("PicartoChannel")).await?;
            let progress = PicartoConfig {
                config: progress.config,
                all_channels: all_channels,
                done_channels: Vec::new(),
                started: true,
            };
            progress
        }
    };
    info!("Loading checkpoint for channel refresh");
    let mut checkpoint = Checkpoint::new();
    checkpoint.set_json(&progress)?;
    for mut channel in progress.all_channels.clone() {
        debug!("Job {}: refreshing channel {}", current_job.id(), channel.id);
        if progress.done_channels.contains(&channel.id) {
            continue;
        }
        match refresh_channel(&progress.config, &mut client, &mut channel).await {
            Ok(_) => {
                progress.done_channels.push(channel.id);
                checkpoint.set_json(&progress)?;
                current_job.checkpoint(&checkpoint).await?;
                debug!("Completed refresh for channel {}", channel.id);
            },
            Err(e) => {
                error!("Failed refresh on channel {} ({:?})", channel.id, channel.short_name);
                current_job.checkpoint(&checkpoint).await?;
            }
        };
    }
    info!("Job {}: Completed refresh", current_job.id());
    current_job.complete().await?;
    Ok(())
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Progress {
    config: Configuration,
    all_channels: Vec<Channel>,
    done_channels: Vec<i32>,
}

#[tracing::instrument]
async fn refresh_channel(
    config: &Configuration,
    client: &mut Client,
    chan: &mut Channel,
) -> TiberiusResult<()> {
    match chan.r#type {
        tiberius_models::ChannelType::PicartoChannel => {
            picarto::refresh_picarto_channel(config, client, chan).await
        }
        tiberius_models::ChannelType::PiczelChannel => todo!(),
        tiberius_models::ChannelType::TwitchChannel => todo!(),
    }
}
