use std::{path::PathBuf, str::FromStr};

use crate::{app::HTTPReq, pages::common::APIMethod};
use anyhow::Result;
use either::Either;
use philomena_models::{Channel, Client, Forum, Image, ImageThumbType, ImageThumbUrl, Tag, User};

pub fn stylesheet_path(req: &HTTPReq) -> Result<String> {
    Ok(if let Some(user) = req.ext::<User>() {
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
        static_path(req, path).to_string_lossy().to_string()
    } else {
        static_path(req, PathBuf::from_str("css/default.css")?)
            .to_string_lossy()
            .to_string()
    })
}

pub fn dark_stylesheet_path(req: &HTTPReq) -> Result<String> {
    Ok(static_path(req, PathBuf::from_str("css/dark.css")?)
        .to_string_lossy()
        .to_string())
}

pub fn api_json_oembed_url(req: &HTTPReq) -> Result<url::Url> {
    path2url(req, "/oembed")
}

pub fn path2url<S: Into<PathBuf>>(req: &HTTPReq, path: S) -> Result<url::Url> {
    use uri_builder::URI;
    let path: PathBuf = path.into();
    let path = path.to_string_lossy().to_string();
    let uri = URI::new(req.url().scheme())
        .host(req.host().unwrap_or("invalid_host_header_fix_your_client"))
        .path(&path.trim_start_matches("/"))
        .build();
    Ok(url::Url::from_str(&uri.to_string())?)
}

pub fn static_path<S: Into<PathBuf>>(_req: &HTTPReq, path: S) -> PathBuf {
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
        crate::assets::Assets::iter().any(|x| x == path.to_string_lossy()),
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

pub async fn thumb_url(
    req: &HTTPReq,
    client: &mut Client,
    img: Either<i64, &Image>,
    thumb: ImageThumbType,
) -> Result<PathBuf> {
    let show_hidden = req.ext::<ShowHidden>().unwrap_or(&ShowHidden(false)).0;
    let image: Image = match img {
        Either::Right(img) => (img.clone()),
        Either::Left(id) => match Image::get(client, id).await? {
            Some(i) => i,
            None => todo!("implement image 404"),
        },
    };
    let date = {
        let date = image.created_at.date();
        date.format("%Y/%m/%d").to_string()
    };
    let deleted = image.hidden_from_users;
    let root = &req.state().config().image_url_root;
    let format = thumb_format_unnamed(image.image_format.map(|x| x.to_lowercase()), false);
    let id_fragment = {
        if deleted && show_hidden {
            format!(
                "{}-{}",
                image.id,
                image
                    .hidden_image_key
                    .expect("hidden key expected but not found")
            )
        } else {
            image.id.to_string()
        }
    };
    let name = thumb.to_string();
    Ok(PathBuf::from_str(&format!(
        "{root}/{date}/{id_fragment}/{name}.{format}",
        root = root,
        date = date,
        id_fragment = id_fragment,
        name = name,
        format = format
    ))?)
}

pub async fn thumb_urls(
    req: &HTTPReq,
    client: &mut Client,
    img: Either<i64, &Image>,
) -> Result<ImageThumbUrl> {
    Ok(ImageThumbUrl {
        rendered: thumb_url(req, client, img, ImageThumbType::Rendered).await?,
        full: thumb_url(req, client, img, ImageThumbType::Full).await?,
        tall: thumb_url(req, client, img, ImageThumbType::Tall).await?,
        large: thumb_url(req, client, img, ImageThumbType::Large).await?,
        medium: thumb_url(req, client, img, ImageThumbType::Medium).await?,
        small: thumb_url(req, client, img, ImageThumbType::Small).await?,
        thumb: thumb_url(req, client, img, ImageThumbType::Thumb).await?,
        thumb_small: thumb_url(req, client, img, ImageThumbType::ThumbSmall).await?,
        thumb_tiny: thumb_url(req, client, img, ImageThumbType::ThumbTiny).await?,
    })
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

#[deprecated]
pub fn todo_path() -> PathBuf {
    PathBuf::from_str("/todo").unwrap()
}

pub fn profile_path_current_user(req: &HTTPReq) -> PathBuf {
    todo_path()
}

pub fn gallery_patch_current_user(req: &HTTPReq) -> PathBuf {
    todo_path()
}

pub fn profile_artist_path_current_user(req: &HTTPReq) -> PathBuf {
    todo_path()
}

pub fn registration_path() -> PathBuf {
    PathBuf::from_str("/registrations/new").expect("new registrations path must be valid")
}

pub fn logout_path() -> PathBuf {
    PathBuf::from_str("/sessions").expect("logout path must be valid")
}

pub fn login_path() -> PathBuf {
    PathBuf::from_str("/sessions/new").expect("new session path must be valid")
}

pub fn forum_route(forum: &Forum) -> Result<PathBuf> {
    Ok(PathBuf::from_str(&format!("/forum/{}", forum.short_name))?)
}

pub fn cdn_host(req: &HTTPReq) -> String {
    let cdn_host = req.state().config.cdn_host.clone();
    cdn_host.unwrap_or(req.host().unwrap_or("this site's domain").to_string())
}

pub fn channel_nsfw_path(method: APIMethod) -> PathBuf {
    todo_path()
}

pub fn channel_route(channel: &Channel) -> PathBuf {
    todo_path()
}

pub fn artist_route(tag: &Tag) -> PathBuf {
    todo_path()
}
