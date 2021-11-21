use std::{path::PathBuf, str::FromStr};

use either::Either;
use rocket::{Request, State};
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::SessionMode;
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Channel, Client, Forum, Image, ImageThumbType, ImageThumbUrl, Tag, User};

pub async fn stylesheet_path<const T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_, T>,
) -> TiberiusResult<String> {
    let user = rstate.user(state).await?;
    Ok(if let Some(user) = user {
        let mut path = PathBuf::from_str("css/")?;
        assert!(
            !user.theme.contains("/"),
            "User theme cannot contain path segments: {:?}",
            user.theme
        );
        path.push(format!("{}.css", user.theme));
        assert!(
            path.is_relative(),
            "user theme path ({:?}) must be relative",
            path
        );
        static_path(path).to_string_lossy().to_string()
    } else {
        static_path(PathBuf::from_str("css/default.css")?)
            .to_string_lossy()
            .to_string()
    })
}

pub fn dark_stylesheet_path<const T: SessionMode>(
    rstate: &TiberiusRequestState<'_, T>,
) -> TiberiusResult<String> {
    Ok(static_path(PathBuf::from_str("css/dark.css")?)
        .to_string_lossy()
        .to_string())
}

pub fn static_path<S: Into<PathBuf>>(path: S) -> PathBuf {
    // Statics are hosted on root, but on a different hash name, where to get?
    let path: PathBuf = path.into();
    assert!(
        path.is_relative(),
        "Must only pass relative paths as assets: {:?}",
        path
    );
    let prefix: PathBuf = PathBuf::from_str("/static").unwrap();
    let path = prefix.join(path);
    assert!(
        tiberius_core::assets::Assets::iter().any(|x| x == path.to_string_lossy()),
        "asset must exist in repository: {:?}",
        path
    );
    path
}

pub fn image_url(img_id: Either<i64, &Image>) -> PathBuf {
    PathBuf::from_str(
        match img_id {
            Either::Left(id) => format!("/{}", id),
            Either::Right(image) => format!("/{}", image.id),
        }
        .as_str(),
    )
    .expect("must have been able to format this")
}

pub struct ShowHidden(pub bool);

#[deprecated]
pub async fn thumb_url<const T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_, { T }>,
    client: &mut Client,
    img: Either<i64, &Image>,
    thumb: ImageThumbType,
) -> TiberiusResult<PathBuf> {
    let show_hidden = true; // TODO: read show hidden from user settings
    let image: Image = match img {
        Either::Right(img) => (img.clone()),
        Either::Left(id) => match Image::get(client, id).await? {
            Some(i) => i,
            None => todo!("implement image 404"),
        },
    };
    let format = thumb_format_unnamed(image.image_format.map(|x| x.to_lowercase()), false);
    let name = thumb.to_string();
    //TODO: replace this!!!
    Ok(PathBuf::from_str(
        &uri!(crate::pages::files::image_thumb_get_simple(
            id = image.id as u64,
            thumbtype = &name,
            _filename = format!("{}.{}", &name, format)
        ))
        .to_string(),
    )
    .unwrap())
}

pub fn thumb_format_unnamed<S: Into<String>>(format: Option<S>, download: bool) -> String {
    thumb_format::<S, String>(format, None, download)
}

pub fn thumb_format<S: Into<String>, R: Into<String>>(
    format: Option<S>,
    name: Option<R>,
    download: bool,
) -> String {
    let format: Option<String> = format.map(|x| x.into());
    let name: Option<String> = name.map(|x| x.into());
    match (format, name, download) {
        (_, Some(rendered), false) => "png".to_string(),
        (Some(format), _, download) => {
            if format == "svg" && !download {
                "png".to_string()
            } else {
                format
            }
        }
        (_, _, _) => "png".to_string(),
    }
}

pub async fn cdn_host<const T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_, { T }>,
) -> String {
    let cdn_host = state.config.cdn_host.clone();
    cdn_host.unwrap_or(
        rstate
            .headers
            .get_one("Host")
            .unwrap_or("this site's domain")
            .to_string(),
    )
}
