use std::str::FromStr;

use async_std::path::PathBuf;
use log::trace;
use rocket::State;
use rocket::http::{ContentType, Status};
use rocket::response::status;
use rocket::response::stream::ReaderStream;
use rocket::response::content;
use rocket::tokio::fs::File;

use crate::config::Configuration;
use crate::error::{TiberiusError, TiberiusResult};

#[get("/img/<year>/<month>/<day>/<id>/<thumbtype>")]
pub async fn image_thumb_get(
    config: State<Configuration>,
    year: u16,
    month: u8,
    day: u8,
    id: u64,
    thumbtype: String,
) -> TiberiusResult<status::Custom<content::Custom<ReaderStream![File]>>> {
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
            ReaderStream::one(file)
        )
    ))
}
