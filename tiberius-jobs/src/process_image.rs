use image::GenericImageView;
use sqlx::{FromRow, Pool, Postgres};
use sqlxmq::{job, Checkpoint, CurrentJob};
use tiberius_core::{config::Configuration, error::TiberiusResult, state::TiberiusState};
use tiberius_dependencies::hex;
use tiberius_models::{Channel, Client, Image, ImageThumbType, Queryable};

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
pub(crate) async fn run_job(mut current_job: CurrentJob, sctx: SharedCtx) -> TiberiusResult<()> {
    info!("Job {}: Processing image", current_job.id());
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
    info!(
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
        info!(
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
        info!(
            "Job {job}: Image {image}: Generate Thumbnails",
            job = current_job.id(),
            image = img.id,
        );
        //TODO: handle GIF animations and APNG (probably want to convert GIF to APNG)

        debug!("Kick off thumbnail jobs");

        let imagef = std::sync::Arc::new(Box::new(imagef));
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
        info!(
            "Job {}: Image {}: Marking Image as Processed",
            current_job.id(),
            img.id
        );
        img.processed = true;
    }
    let img = img.save(&mut client).await?;
    info!(
        "Job {}: Image {}: Processing step persisted to database",
        current_job.id(),
        img.id
    );
    info!(
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
    info!(
        "Job {}: Processing complete in {:4.3} seconds!",
        current_job.id(),
        time_spent
    );
    Ok(())
}

pub async fn make_thumb(
    img: std::sync::Arc<Box<image::DynamicImage>>,
    thumb_size: ImageThumbType,
) -> TiberiusResult<image::DynamicImage> {
    let res = thumb_size.to_resolution_limit();
    // TODO: handle gif, maybe
    Ok(match res {
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
                img.thumbnail(res.width, res.height)
            })
            .await?
        }
        None => (**img).clone(),
    })
}
