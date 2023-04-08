use std::{path::PathBuf, str::FromStr};

use axum::headers::{HeaderMapExt, Host};
use either::Either;
use tiberius_core::{
    error::TiberiusResult,
    session::SessionMode,
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_models::{Channel, Client, Forum, Image, ImageThumbType, ImageThumbUrl, Tag, User};

pub async fn stylesheet_path<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
) -> TiberiusResult<String> {
    let user = rstate.user(state).await?;
    Ok(if let Some(user) = user {
        let mut path = PathBuf::from_str("css/")?;
        assert!(
            !user.user_settings.theme.contains('/'),
            "User theme cannot contain path segments: {:?}",
            user.user_settings.theme
        );
        path.push(format!("{}.css", user.user_settings.theme));
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

pub fn dark_stylesheet_path<T: SessionMode>(
    rstate: &TiberiusRequestState<T>,
) -> TiberiusResult<String> {
    Ok(static_path(PathBuf::from_str("css/dark.css")?)
        .to_string_lossy()
        .to_string())
}

#[instrument(skip(path))]
pub fn static_path<S: Into<PathBuf>>(path: S) -> PathBuf {
    // Statics are hosted on root, but on a different hash name, where to get?
    let path: PathBuf = path.into();
    assert!(
        path.is_relative(),
        "Must only pass relative paths as assets: {:?}",
        path
    );
    let prefix: PathBuf = PathBuf::from_str("/static").unwrap();
    let prefix = prefix.join(path.clone());
    let path = PathBuf::from_str(&format!("/{}", path.display())).unwrap();
    assert!(
        tiberius_core::assets::Assets::iter().any(|x| x == path.to_string_lossy()),
        "asset must exist in repository: {:?} ({:?})",
        path,
        prefix
    );
    prefix
}

pub struct ShowHidden(pub bool);

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

pub async fn cdn_host<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
) -> String {
    let cdn_host = state.config.cdn_host.clone();
    cdn_host.unwrap_or(
        rstate
            .headers
            .typed_get::<Host>()
            .map(|x| x.to_string())
            .unwrap_or("invalid or missing host header".to_string()),
    )
}
