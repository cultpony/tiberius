use std::str::FromStr;

use async_std::path::PathBuf;
use axum::{
    extract::State,
    headers::{ContentType, HeaderMapExt},
    http::{HeaderMap, StatusCode},
    Extension, Router,
};
use axum_extra::{
    body::AsyncReadBody,
    routing::{RouterExt, TypedPath},
};
use lazy_static::lazy_static;
use new_mime_guess::Mime;
use regex::Regex;
use tiberius_core::{
    config::Configuration,
    error::{TiberiusError, TiberiusResult},
    request_helper::{CustomResponse, TiberiusResponse},
    session::Unauthenticated,
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_dependencies::chrono::Datelike;
use tiberius_dependencies::mime;
use tiberius_models::{Client, Image, ImageThumbType, PathImageGetFull, PathImageThumbGet};
use tokio::fs::File;
use tracing::trace;

pub fn static_file_pages(r: Router<TiberiusState>) -> Router<TiberiusState> {
    r.typed_get(image_thumb_get)
        .typed_get(image_thumb_get_simple)
        .typed_get(image_full_get)
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/img/thumb/:id/:thumbtype/:filename")]
pub struct PathImageThumbGetSimple {
    pub id: u64,
    pub thumbtype: String,
    pub filename: String,
}

#[instrument]
pub async fn image_thumb_get_simple(
    PathImageThumbGetSimple {
        id,
        thumbtype,
        filename: _filename,
    }: PathImageThumbGetSimple,
    State(state): State<TiberiusState>,
) -> TiberiusResult<TiberiusResponse<AsyncReadBody<File>>> {
    let mut client = state.get_db_client();
    let image = Image::get_id(&mut client, id as i64).await?;
    match image {
        None => Err(TiberiusError::Other(
            "Could not find image thumb".to_string(),
        )),
        Some(image) => {
            let path = PathBuf::from(image.thumbnail_path(ImageThumbType::Small).await?);
            let config = state.config();
            Ok(read_static(State(state), &path, Some(image)).await?)
        }
    }
}

#[derive(TypedPath, serde::Deserialize)]
#[typed_path("/img/view/:year/:month/:day/:filename")]
pub struct PathImageGetShort {
    pub filename: String,
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl From<&Image> for PathImageGetShort {
    fn from(i: &Image) -> Self {
        Self {
            filename: i.filename(),
            year: i.created_at.year() as u16,
            month: i.created_at.month() as u8,
            day: i.created_at.day() as u8,
        }
    }
}

lazy_static! {
    static ref FILENAME_REGEX: Regex = Regex::new(r#"(?P<id>\d+)(__.+)?(?P<ext>\.\w+)"#).unwrap();
}

#[instrument]
pub async fn image_full_get(
    PathImageGetFull {
        filename,
        year,
        month,
        day,
    }: PathImageGetFull,
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<AsyncReadBody<File>>> {
    let config = &state.config;
    trace!("using full path image");
    let mut client = state.get_db_client();
    let parsed_filename = match FILENAME_REGEX.captures(&filename) {
        None => {
            return Err(TiberiusError::PageNotFound(format!(
                "Could not find image {filename}"
            )))
        }
        Some(parsed_filename) => parsed_filename,
    };
    let id: i64 = parsed_filename.name("id").unwrap().as_str().parse()?;
    let ext = parsed_filename.name("ext").unwrap().as_str();
    let image = Image::get_id(&mut client, id).await?;
    let path = if let Some(image) = image {
        if let Some(image_path) = image.image {
            let path = PathBuf::from_str(&image_path)?;
            let path = PathBuf::from_str("images")?.join(path);

            config
                .data_root
                .clone()
                .expect("require static data root")
                .join(path)
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
        .unwrap_or(tiberius_dependencies::mime::IMAGE_PNG);
    let file = File::open(path).await?;
    let mut hm = HeaderMap::new();
    hm.typed_insert(ContentType::from(mime));
    Ok(TiberiusResponse::Custom(CustomResponse {
        content: AsyncReadBody::new(file),
        headers: hm,
    }))
}

#[instrument]
pub async fn image_thumb_get(
    PathImageThumbGet {
        year,
        month,
        day,
        id,
        filename,
    }: PathImageThumbGet,
    State(state): State<TiberiusState>,
) -> TiberiusResult<TiberiusResponse<AsyncReadBody<File>>> {
    let config = &state.config;
    let path = format!("images/thumbs/{year}/{month}/{day}/{id}/{filename}",);
    let mut client = state.get_db_client();
    let image = Image::get_id(&mut client, id as i64).await?;
    read_static(State(state), &PathBuf::from(path), image).await
}

#[instrument]
async fn read_static(
    State(state): State<TiberiusState>,
    path: &PathBuf,
    image: Option<Image>,
) -> TiberiusResult<TiberiusResponse<AsyncReadBody<File>>> {
    let config = state.config();
    trace!("requesting static file {}", path.display());
    let path = config
        .data_root
        .clone()
        .expect("require static data root")
        .join(path);
    let path = if let Ok(md) = path.symlink_metadata() {
        if md.file_type().is_symlink() {
            trace!("using full path image");
            let client = state.get_db_client();
            let image = image;
            if let Some(image) = image {
                if let Some(image_path) = image.image {
                    let path = PathBuf::from_str(&image_path)?;
                    let path = PathBuf::from_str("images")?.join(path);
                    config
                        .data_root
                        .clone()
                        .expect("require static data root")
                        .join(path)
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
    let mime = mime.first().unwrap_or(mime::IMAGE_PNG);
    let file = File::open(path).await?;
    let mut hm = HeaderMap::new();
    hm.typed_insert(ContentType::from(mime));
    Ok(TiberiusResponse::Custom(CustomResponse {
        content: AsyncReadBody::new(file),
        headers: hm,
    }))
}
