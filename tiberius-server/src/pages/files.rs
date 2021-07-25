use std::str::FromStr;

use async_std::path::PathBuf;
use chrono::Datelike;
use log::trace;
use tiberius_core::error::{TiberiusError, TiberiusResult};
use tiberius_core::request_helper::{CustomResponse, TiberiusResponse};
use tiberius_core::state::TiberiusState;
use tiberius_models::Image;
use rocket::http::{ContentType, Status};
use rocket::response::content;
use rocket::response::status;
use rocket::response::stream::ReaderStream;
use rocket::tokio::fs::File;
use rocket::State;

#[get("/img/thumb/<id>/<thumbtype>/<_filename>")]
pub async fn image_thumb_get_simple(
    state: &State<TiberiusState>,
    id: u64,
    thumbtype: String,
    _filename: String
) -> TiberiusResult<status::Custom<content::Custom<ReaderStream![File]>>> {
    let mut client = state.get_db_client().await?;
    let image = Image::get_id(&mut client, id as i64).await?;
    match image {
        None => Err(TiberiusError::Other(
            "Could not find image thumb".to_string(),
        )),
        Some(image) => {
            let created = image.created_at;
            let year = created.year();
            let month = created.month();
            let day = created.day();
            Ok(image_thumb_get(
                state,
                year as u16,
                month as u8,
                day as u8,
                id,
                format!(
                    "{}.{}",
                    thumbtype,
                    image.image_format.unwrap_or("png".to_string())
                ),
                "".to_string(),
            )
            .await?)
        }
    }
}

#[get("/img/view/<id>/<_filename>")]
pub async fn image_full_get(
    state: &State<TiberiusState>,
    id: u64,
    _filename: String,
) -> TiberiusResult<TiberiusResponse<ReaderStream<rocket::response::stream::One<tokio::fs::File>>>>
{
    let config = &state.config;
    trace!("using full path image");
    let mut client = state.get_db_client().await?;
    let image = Image::get_id(&mut client, id as i64).await?;
    let path = if let Some(image) = image {
        if let Some(image_path) = image.image {
            let path = PathBuf::from_str(&image_path)?;
            let path = PathBuf::from_str("images")?.join(path);
            let path = config.data_root.clone().join(path);
            path
        } else {
            return Ok(TiberiusResponse::Error(TiberiusError::Other(
                "Image not found".to_string(),
            )));
        }
    } else {
        return Ok(TiberiusResponse::Error(TiberiusError::Other(
            "Image not found".to_string(),
        )));
    };
    trace!("full static file path: {}", path.display());
    let mime = new_mime_guess::from_path(path.clone());
    let mime = mime
        .first()
        .map(|x| x.essence_str().to_string())
        .unwrap_or("image/png".to_string());
    let file = File::open(path).await?;
    Ok(TiberiusResponse::Custom(CustomResponse {
        content: ReaderStream::one(file),
        content_type: ContentType::from_str(&mime).map_err(|x| TiberiusError::Other(x))?,
    }))
}

#[get("/img/thumb/<year>/<month>/<day>/<id>/<thumbtype>/<_filename>")]
pub async fn image_thumb_get(
    state: &State<TiberiusState>,
    year: u16,
    month: u8,
    day: u8,
    id: u64,
    thumbtype: String,
    _filename: String,
) -> TiberiusResult<status::Custom<content::Custom<ReaderStream![File]>>> {
    let config = &state.config;
    let path = format!(
        "images/thumbs/{year}/{month}/{day}/{id}/{thumbtype}",
        year = year,
        month = month,
        day = day,
        id = id,
        thumbtype = thumbtype
    );
    trace!("requesting static file {}", path);
    let path = PathBuf::from_str(&path)?;
    let path = config.data_root.clone().join(path);
    let path = if let Ok(md) = path.symlink_metadata() {
        if md.file_type().is_symlink() {
            trace!("using full path image");
            let mut client = state.get_db_client().await?;
            let image = Image::get_id(&mut client, id as i64).await?;
            if let Some(image) = image {
                if let Some(image_path) = image.image {
                    let path = PathBuf::from_str(&image_path)?;
                    let path = PathBuf::from_str("images")?.join(path);
                    let path = config.data_root.clone().join(path);
                    path
                } else {
                    path
                }
            } else {
                path
            }
        } else {
            path
        }
    } else {
        path
    };
    trace!("full static file path: {}", path.display());
    let mime = new_mime_guess::from_path(path.clone());
    let mime = mime
        .first()
        .map(|x| x.essence_str().to_string())
        .unwrap_or("image/png".to_string());
    let file = File::open(path).await?;
    Ok(status::Custom(
        Status::Ok,
        content::Custom(
            ContentType::from_str(&mime).map_err(|x| TiberiusError::Other(x))?,
            ReaderStream::one(file),
        ),
    ))
}
