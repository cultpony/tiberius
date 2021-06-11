use std::str::FromStr;

use async_std::path::PathBuf;
use log::trace;

use crate::app::HTTPReq;

pub async fn image_thumb_get(req: HTTPReq) -> tide::Result {
    let year: u16 = req.param("year")?.parse()?;
    let month: u8 = req.param("month")?.parse()?;
    let day: u8 = req.param("day")?.parse()?;
    let id: u64 = req.param("id")?.parse()?;
    let thumbtype = req.param("thumbtype")?;

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
    let path = req.state().config().data_root.clone().join(path);
    trace!("full static file path: {}", path.display());
    let mime = new_mime_guess::from_path(path.clone());
    let mime = mime
        .first()
        .map(|x| x.essence_str().to_string())
        .unwrap_or("image/png".to_string());
    let body = tide::Body::from_file(path).await?;
    Ok(tide::Response::builder(200)
        .content_type(&*mime)
        .body(body)
        .build())
}
