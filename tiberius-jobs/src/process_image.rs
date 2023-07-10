use tiberius_core::{config::Configuration, error::TiberiusResult, state::TiberiusState};
use tiberius_dependencies::hex;
use tiberius_dependencies::image;
use tiberius_dependencies::image::GenericImageView;
use tiberius_dependencies::prelude::*;
use tiberius_dependencies::sentry;
use tiberius_dependencies::serde;
use tiberius_dependencies::serde_json;
use tiberius_dependencies::sha2;
use tiberius_dependencies::sqlx;
use tiberius_dependencies::sqlx::{FromRow, Pool, Postgres};
use tiberius_dependencies::sqlxmq;
use tiberius_dependencies::sqlxmq::{job, Checkpoint, CurrentJob};
use tiberius_dependencies::tokio;
use tiberius_models::{Channel, Client, Image, ImageThumbType, Queryable};

use crate::generate_thumbnails::make_thumb;
use crate::SharedCtx;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct ImageProcessConfig {
    pub image_id: u64,
}

pub async fn process_image<'a, E: sqlx::Executor<'a, Database = sqlx::Postgres>>(
    executor: E,
    ipc: ImageProcessConfig,
) -> TiberiusResult<()> {
    run_job.builder().set_json(&ipc)?.spawn(executor).await?;
    Ok(())
}

#[instrument(skip(current_job, sctx))]
#[sqlxmq::job(retries = 3, backoff_secs = 10)]
pub(crate) async fn run_job(current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    sentry::configure_scope(|scope| {
        scope.clear();
    });
    let tx = sentry::start_transaction(sentry::TransactionContext::new(
        "process_image",
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
async fn tx_run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    debug!("Job {}: Processing image", current_job.id());
    let start = std::time::Instant::now();
    let pool = current_job.pool();
    let progress: ImageProcessConfig = current_job
        .json()?
        .expect("job requires configuration copy");
    let mut client = sctx.client;
    let img = Image::get(&mut client, progress.image_id as i64).await?;
    let mut img = match img {
        Some(v) => v,
        None => {
            error!("Job {}: Failed: No Image", current_job.id());
            current_job.complete().await?;
            return Ok(());
        }
    };
    debug!(
        "Job {}: Image {}: Rewrite Image Data (Sanity Filter)",
        current_job.id(),
        img.id
    );
    let dataroot = sctx
        .config
        .data_root
        .clone()
        .expect("require configured data root directory");
    //TODO: improve error handling here
    {
        let path = dataroot.join("images").join(img.image.clone().unwrap());
        assert!(
            path.exists(),
            "Path to newly uploaded image must exist: {}",
            path.display()
        );
    }
    let imagef = {
        debug!(
            "Job {}: Image {}: Update Metadata",
            current_job.id(),
            img.id
        );
        let imagefm = img.statf(&dataroot).await?;
        img.image_sha512_hash = {
            let mut imagef = img.openf(&dataroot).await?;
            use sha2::Digest;
            let hash = tokio::task::spawn_blocking(move || -> TiberiusResult<sha2::Sha512> {
                let mut hasher = sha2::Sha512::new();
                std::io::copy(&mut imagef, &mut hasher)?;
                Ok(hasher)
            })
            .await??;
            let res = hash.finalize();
            Some(hex::encode(&res[..]))
        };
        img.image_orig_sha512_hash = img.image_sha512_hash.clone();
        let imagef = img.openf(&dataroot).await?;
        let imagef = image::io::Reader::new(imagef).with_guessed_format()?;
        let imageff = imagef.format();
        debug!("Job {}: Found image format {:?}", current_job.id(), imageff);
        let imagef: image::DynamicImage = imagef.decode()?;
        assert!(imagef.height() < i32::MAX as u32, "Image too large");
        assert!(imagef.width() < i32::MAX as u32, "Image too large");
        img.image_height = Some(imagef.height() as i32);
        img.image_width = Some(imagef.width() as i32);
        img.image_aspect_ratio = Some(imagef.width() as f64 / imagef.height() as f64);
        assert!(imagefm.len() < i32::MAX as u64);
        img.image_size = Some(imagefm.len() as i32);
        //TODO: on videos, get real duration, also handle GIFs here
        img.image_duration = None;
        img.image_format = imageff.map(|x| format!("{:?}", x).to_lowercase());
        imagef
    };
    {
        debug!(
            "Job {job}: Image {image}: Generate Thumbnails",
            job = current_job.id(),
            image = img.id,
        );
        //TODO: handle GIF animations and APNG (probably want to convert GIF to APNG)

        debug!("Kick off thumbnail jobs");

        let imagef = std::sync::Arc::new(imagef);
        let large = make_thumb(imagef.clone(), ImageThumbType::Large);
        let medium = make_thumb(imagef.clone(), ImageThumbType::Medium);
        let small = make_thumb(imagef.clone(), ImageThumbType::Small);
        let thumb = make_thumb(imagef.clone(), ImageThumbType::Thumb);

        debug!("Constructing filesystem layout");

        let base = dataroot.join(img.thumbnail_basepath().await?);

        debug!("fs layout: {base_path}", base_path = base.display());

        std::fs::create_dir_all(base)?;

        let largep = dataroot.join(img.thumbnail_path(ImageThumbType::Large).await?);
        let mediump = dataroot.join(img.thumbnail_path(ImageThumbType::Medium).await?);
        let smallp = dataroot.join(img.thumbnail_path(ImageThumbType::Small).await?);
        let thumbp = dataroot.join(img.thumbnail_path(ImageThumbType::Thumb).await?);

        debug!("Generated img paths, large = {}", largep.display());
        debug!("Generated img paths, medium = {}", mediump.display());
        debug!("Generated img paths, small = {}", smallp.display());
        debug!("Generated img paths, thumb = {}", thumbp.display());

        let (large, medium, small, thumb) = tokio::join!(large, medium, small, thumb);
        let (large, medium, small, thumb) = (large?, medium?, small?, thumb?);

        tokio::task::spawn_blocking(move || -> TiberiusResult<()> {
            large.save(largep)?;
            medium.save(mediump)?;
            small.save(smallp)?;
            thumb.save(thumbp)?;
            Ok(())
        })
        .await??;

        img.thumbnails_generated = true;
    }
    {
        debug!(
            "Job {}: Image {}: Marking Image as Processed",
            current_job.id(),
            img.id
        );
        img.processed = true;
    }
    let img = img.save(&mut client).await?;
    debug!(
        "Job {}: Image {}: Processing step persisted to database",
        current_job.id(),
        img.id
    );
    debug!(
        "Job {}: Image {}: Scheduling Reindex",
        current_job.id(),
        img.id
    );
    let reindex_config = crate::reindex_images::ImageReindexConfig {
        image_ids: Some(vec![img.id as i64]),
        ..Default::default()
    };
    crate::reindex_images::reindex_images(pool, reindex_config).await?;
    current_job.complete().await?;
    let end = std::time::Instant::now();
    let time_spent = end - start;
    let time_spent = time_spent.as_secs_f32();
    debug!(
        "Job {}: Processing complete in {:4.3} seconds!",
        current_job.id(),
        time_spent
    );
    Ok(())
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use tiberius_core::error::TiberiusResult;
    use tiberius_dependencies::image;
    use tiberius_dependencies::tokio;
    use tiberius_dependencies::tokio::io::AsyncReadExt;
    use tiberius_models::ImageThumbType;


    #[tokio::test]
    #[ignore = "need to find suitable test data"]
    async fn test_very_large_gif_image() -> TiberiusResult<()> {
        let image_path = "../test_data/very_large_gif.gif";
        let f = image::ImageFormat::Gif;
        let start = std::time::Instant::now();
        let r = tokio::fs::File::open(image_path).await?;
        let mut r = tokio::io::BufReader::new(r);
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).await?;
        println!(
            "Took {:.5} seconds to read image in format {f:?}",
            start.elapsed().as_secs_f32()
        );
        let img = image::load_from_memory_with_format(&buf, f)?;
        println!(
            "Took {:.5} seconds to load image",
            start.elapsed().as_secs_f32()
        );
        let img = Arc::new(img);

        // Test Small Thumb
        let thumb_type = ImageThumbType::Small;
        testfun_make_thumb(ImageThumbType::Rendered, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Full, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Tall, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Large, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Medium, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Small, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Thumb, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::ThumbSmall, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::ThumbTiny, img.clone()).await?;

        println!(
            "Took {:.5} seconds to process image static thumbs",
            start.elapsed().as_secs_f32()
        );
        assert!(false);

        Ok(())
    }

    #[tokio::test]
    async fn test_very_tall_image() -> TiberiusResult<()> {
        let image_path = "../test_data/very_tall_image_conversion.jpg";
        let f = image::ImageFormat::Jpeg;
        let start = std::time::Instant::now();
        let r = tokio::fs::File::open(image_path).await?;
        let mut r = tokio::io::BufReader::new(r);
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).await?;
        println!(
            "Took {:.5} seconds to read image",
            start.elapsed().as_secs_f32()
        );
        let img = image::load_from_memory_with_format(&buf, f)?;
        assert!(
            img.height() >= 20000,
            "Image should be atleast 20k pixels tall, was {}",
            img.height()
        );
        assert!(
            img.width() >= 1500,
            "Image should be atleast 1500 pixels wide, was {}",
            img.width()
        );
        println!(
            "Took {:.5} seconds to load image",
            start.elapsed().as_secs_f32()
        );
        let img = Arc::new(img);

        // Test Small Thumb
        let thumb_type = ImageThumbType::Small;
        testfun_make_thumb(ImageThumbType::Rendered, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Full, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Tall, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Large, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Medium, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Small, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::Thumb, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::ThumbSmall, img.clone()).await?;
        testfun_make_thumb(ImageThumbType::ThumbTiny, img.clone()).await?;

        Ok(())
    }

    async fn testfun_make_thumb(
        thumb_type: ImageThumbType,
        img: Arc<image::DynamicImage>,
    ) -> TiberiusResult<()> {
        println!(
            "Clamping to {:?}, {:?}",
            thumb_type,
            thumb_type.to_resolution_limit()
        );
        println!(
            "Expected resolution: {:?}",
            thumb_type
                .to_resolution_limit()
                .map(|x| x.clamp_resolution(img.height(), img.width()))
        );
        let start = std::time::Instant::now();
        let thumb = crate::generate_thumbnails::make_thumb(img, thumb_type).await?;
        let elapsed = start.elapsed();
        println!("Took {:.5} seconds", elapsed.as_secs_f32());
        println!(
            "Real resolution: height: {:?}, width: {:?}",
            thumb.height(), thumb.width()
        );
        assert!(
            thumb.width()
                <= thumb_type
                    .to_resolution_limit()
                    .map(|x| x.width)
                    .unwrap_or(thumb.width()),
            "Thumbnail Width should have been clamped to {} but was clamped to {}",
            thumb_type.to_resolution_limit().unwrap().width,
            thumb.width()
        );
        assert!(
            thumb.height()
                <= thumb_type
                    .to_resolution_limit()
                    .map(|x| x.height)
                    .unwrap_or(thumb.height()),
            "Thumbnail Height should have been clamped to {} but was clamped to {}",
            thumb_type.to_resolution_limit().unwrap().height,
            thumb.height()
        );
        Ok(())
    }
}
