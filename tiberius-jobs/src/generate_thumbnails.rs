use std::path::PathBuf;
use std::sync::Arc;

use crate::SharedCtx;
use tiberius_core::error::TiberiusResult;
use tiberius_dependencies::image;
use tiberius_dependencies::prelude::*;
use tiberius_dependencies::sentry;
use tiberius_dependencies::serde;
use tiberius_dependencies::serde_json;
use tiberius_dependencies::sqlx;
use tiberius_dependencies::sqlxmq;
use tiberius_dependencies::sqlxmq::CurrentJob;
use tiberius_dependencies::tokio;
use tiberius_models::{Image, ImageThumbType};

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct GenerateThumbnailConfig {
    pub image_id: u64,
}

pub async fn generate_thumbnails<'a, E: sqlx::Executor<'a, Database = sqlx::Postgres>>(
    executor: E,
    gtc: GenerateThumbnailConfig,
) -> TiberiusResult<()> {
    run_job.builder().set_json(&gtc)?.spawn(executor).await?;
    Ok(())
}

#[instrument(skip(img))]
pub async fn make_thumb(
    img: std::sync::Arc<image::DynamicImage>,
    thumb_size: ImageThumbType,
) -> TiberiusResult<Box<image::DynamicImage>> {
    let res = thumb_size.to_resolution_limit();
    // TODO: handle gif, maybe
    Ok(Box::new(match res {
        Some(res) => {
            tokio::task::spawn_blocking(move || {
                let res = res.clamp_resolution(img.height(), img.width());
                debug!(
                    "Clamping image from {}, {} -> {}, {}",
                    img.width(),
                    img.height(),
                    res.width,
                    res.height,
                );
                img.thumbnail_exact(res.width, res.height)
            })
            .await?
        }
        None => (*img).clone(),
    }))
}

#[instrument(skip(current_job, sctx))]
#[sqlxmq::job(retries = 0, backoff_secs = 10)]
pub(crate) async fn run_job(current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    sentry::configure_scope(|scope| {
        scope.clear();
    });
    let tx = sentry::start_transaction(sentry::TransactionContext::new(
        "generate_thumbnails",
        "queue.task",
    ));
    match tx_run_job(current_job, sctx).await {
        Ok(()) => {
            tx.set_status(sentry::protocol::SpanStatus::Ok);
            tx.finish();
            Ok(())
        }
        Err(e) => {
            tx.set_status(sentry::protocol::SpanStatus::InternalError);
            tx.set_data("error_msg", serde_json::Value::String(e.to_string()));
            tx.finish();
            Err(e)
        }
    }
}

#[instrument(skip(current_job, sctx))]
async fn tx_run_job(current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    let start = std::time::Instant::now();
    let pool = current_job.pool();
    let progress: GenerateThumbnailConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    let configuration = sctx.config.clone();
    let mut client = sctx.client;
    let img = Image::get(&mut client, progress.image_id as i64)
        .await?
        .expect("start thumb job for image that does not exist");
    let img_db = img.clone();
    debug!(
        "Job {}: Generating Thumbnails for {}",
        current_job.id(),
        img.id
    );
    let thumbs = img.image_thumb_urls().await?;
    let format = match img.image_format.expect("need image format").as_str() {
        "png" => image::ImageFormat::Png,
        "jpg" | "jpeg" => image::ImageFormat::Jpeg,
        "gif" => todo!(),
        _ => image::ImageFormat::Jpeg,
    };

    let img = sctx.config.image_base().join(
        img.image
            .expect("can't generate thumbs for unstored images"),
    );

    let img = std::fs::File::open(img)?;
    let img = std::io::BufReader::new(img);

    let img = tokio::task::spawn_blocking(move || image::load(img, format)).await??;

    let img = Arc::new(img);

    async fn do_make_thumb(
        thumb_type: ImageThumbType,
        store_to: PathBuf,
        format: image::ImageFormat,
        img: Arc<image::DynamicImage>,
    ) -> TiberiusResult<()> {
        debug!(
            "Clamping image to {:?}, {:?}",
            thumb_type,
            thumb_type.to_resolution_limit()
        );
        debug!(
            "Expected resolution: {:?}",
            thumb_type
                .to_resolution_limit()
                .map(|x| x.clamp_resolution(img.height(), img.width()))
        );
        let start = std::time::Instant::now();
        let thumb = make_thumb(img, thumb_type).await?;
        debug!(
            "Took {:.3} sec to process image thumb {:?}",
            start.elapsed().as_secs_f32(),
            thumb_type
        );
        debug!("Saving thumb to {store_to:?}...");
        tokio::task::spawn_blocking(move || {
            //todo!("saved to {store_to:?}");
            thumb.save_with_format(store_to, format)
        })
        .await??;
        debug!(
            "Processing for thumb {:?} complete in {:.3} seconds",
            thumb_type,
            start.elapsed().as_secs_f32()
        );
        Ok(())
    }

    let basepath = configuration
        .image_base()
        .join("thumbs")
        .join(thumbs.large.path().trim_start_matches("/img/"));
    debug!("Checking for basepath of {basepath:?}");
    let basepath = basepath
        .parent()
        .expect("image path cannot be a root directory");

    if !basepath.exists() {
        debug!("Creating directory {basepath:?}");
        std::fs::create_dir_all(basepath)?;
    }

    let (large, medium, small, thumb) = tokio::join!(
        do_make_thumb(
            ImageThumbType::Large,
            configuration
                .image_base()
                .join("thumbs")
                .join(thumbs.large.path().trim_start_matches("/img/")),
            format,
            img.clone()
        ),
        do_make_thumb(
            ImageThumbType::Medium,
            configuration
                .image_base()
                .join("thumbs")
                .join(thumbs.medium.path().trim_start_matches("/img/")),
            format,
            img.clone()
        ),
        do_make_thumb(
            ImageThumbType::Small,
            configuration
                .image_base()
                .join("thumbs")
                .join(thumbs.small.path().trim_start_matches("/img/")),
            format,
            img.clone()
        ),
        do_make_thumb(
            ImageThumbType::Thumb,
            configuration
                .image_base()
                .join("thumbs")
                .join(thumbs.thumb.path().trim_start_matches("/img/")),
            format,
            img.clone()
        ),
    );

    large?;
    medium?;
    small?;
    thumb?;

    // todo improve path replacement
    let full = configuration
        .image_base()
        .join("thumbs")
        .join(thumbs.full_thumbnail.path().trim_start_matches("/img/"));
    let target = PathBuf::from("/srv/philomena/priv/static/system/images/")
        .join(img_db.image.as_ref().unwrap());
    debug!("Symlinking full res from {target:?} to {full:?}");
    // todo this shouldn't be hard coded
    if full.is_symlink() {
        debug!("Symlink exists, removing and readding it");
        std::fs::remove_file(&full)?;
    }
    std::os::unix::fs::symlink(target, full)?;

    debug!("Finished processing, marking good in the database");

    img_db.mark_thumbnails_generated(&mut client).await?;

    Ok(())
}
