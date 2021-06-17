use std::{borrow::Cow, path::PathBuf};

use maud::{html, Markup, Render};
use philomena_models::{Client, Image, ImageThumbType};
use rocket::Request;

use crate::{app::HTTPReq, error::TiberiusResult, pages::common::{
        pagination::PaginationCtl,
        routes::{image_url, thumb_url},
    }};

#[derive(Debug, Clone, Copy)]
pub enum ImageSize {
    Large,
    Medium,
    Small,
}

impl Into<ImageThumbType> for ImageSize {
    fn into(self) -> ImageThumbType {
        use ImageSize::*;
        match self {
            Large => ImageThumbType::Large,
            Medium => ImageThumbType::Medium,
            Small => ImageThumbType::Small,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HeaderSize {
    ThumbSmall,
    None,
}

#[derive(Debug)]
pub enum RenderIntent {
    HiDpi {
        small_url: String,
        medium_url: String,
        hover_text: String,
    },
    // Deprecate this in favor of pure HiDpi rendering, should be a CSS rule
    Image {
        small_url: String,
        medium_url: String,
        hover_text: String,
        webm: bool,
    },
    VideoThumb {
        small_url: String,
        hover_text: String,
    },
    Video {
        webm: String,
        mp4: String,
        hover_text: String,
    },
    FilteredImage {
        hover_text: String,
    },
    FilteredVideo {
        hover_text: String,
    },
    NotRendered,
}

#[derive(Clone, Copy)]
pub enum ImageBlockHeader{
    Small,
    Default,
}

impl ToString for ImageBlockHeader {
    fn to_string(&self) -> String {
        match self {
            Self::Small => "media-box__header--small",
            Self::Default => "",
        }
    }
}

pub async fn image_block_default_sort<S: Into<String>, S1: Into<String>>(
    req: &Request<'_>,
    client: &mut Client,
    query: S,
    page: u64,
    page_size: u64,
    filter_title: S1,
) -> TiberiusResult<Markup> {
    image_block::<S, &str, &str, S1>(
        req,
        client,
        query,
        None,
        None,
        page,
        page_size,
        filter_title,
    )
    .await
}

pub async fn image_block<S1: Into<String>, S2: Into<String>, S3: Into<String>, S4: Into<String>>(
    req: Request<'_>,
    client: &mut Client,
    header: ImageBlockHeader,
    query: S1,
    sort_by: Option<S2>,
    order_by: Option<S3>,
    page: u64,
    page_size: u64,
    filter_title: S4,
) -> TiberiusResult<Markup> {
    let (total, images) = Image::search(client, query, sort_by, order_by, page, page_size).await?;
    let pagination = PaginationCtl::new(
        req,
        &["q", "sf", "sd"],
        total,
        "images",
        "image",
        filter_title,
    )?;
    Ok(html! {
        .block#imagelist-container {
            section.block__header.page__header.flex {
                span.block__header__title.page_title.hide-mobile {
                    (header.to_string())
                }
                .page__pagination { (pagination.pagination()) }

                .flex__right.page__info {
                    //TODO: random button
                    //TODO: hidden toggle
                    //TODO: deleted toggle
                    //TODO: quick tag
                }
            }
            //TODO: info_row

            .block__content.js-resizable-media-container {
                @for image in images {
                    (image_box(req, client, image, ImageSize::Medium, HeaderSize::ThumbSmall).await?)
                }
            }

            .block__header.block__header--light.page__header.flex {
                .page_pagination { (pagination.pagination()) }

                span.block__header__title.page_info {
                    //TODO: render pagination info
                }

                .flex__right.page__options {
                    a href="/settings/edit" title="Display Settings" {
                        i.fa.fa-cog {}
                        span.hide-mobile.hide-limited-desktop {
                            "Display Settings"
                        }
                    }
                }
            }
        }
    })
}

pub async fn image_box<'a>(
    req: &Request<'a>,
    client: &mut Client,
    image: Image,
    image_size: ImageSize,
    header_size: HeaderSize,
) -> TiberiusResult<Markup> {
    let size_class = match image_size {
        ImageSize::Large => "media-box__content--large",
        ImageSize::Medium => "media-box__content--featured",
        ImageSize::Small => "media-box__content--small",
    };
    let header_class = match header_size {
        HeaderSize::ThumbSmall => "media-box__header--small",
        HeaderSize::None => "",
    };
    let interactions = html!{
        a.interaction--fave href="#" rel="nofollow" data-image-id=(image.id) {
            span.fave-span title="Fave!" {
                i.fa.fa-star {}
            }
            span.favorites title="Favorites" data-image-id=(image.id) { (image.faves_count) }
        }
        a.interaction--upvote href="#" rel="nofollow" data-image-id=(image.id) {
            i.fa.fa-arrow-up title="Yay!" {}
        }
        span.score title="Score" data-image-id=(image.id) { (image.score) }
        a.interaction--downvote href="#" rel="nofollow" data-image-id=(image.id) {
            i.fa.fa-arrow-down title="Neigh!" {}
        }
        a.interaction--comments href=(format!("/{}#comments", image.id)) title="Comments" {
            i.fa.fa-comments {}
            span.comments-count data-image-id=(image.id) { (image.comments_count) }
        }
        a.interaction--hide href="#" rel="nofollow" data-image-id=(image.id) {
            i.fa.fa-eye-slash title="Hide" {}
        }
    };
    Ok(html! {
        .media-box data-image-id=(image.id) {
            .media-box__header.media-box__header--link-row.(header_class) data-image-id=(image.id) {
                (interactions)
            }
            .media-box__content.flex.flex--centered.flex--centered-distributed.(size_class) {
                (image_container(req, client, image, image_size.into()).await?)
            }
        }
    })
}

pub async fn image_container<'a>(
    req: &Request<'a>,
    client: &mut Client,
    image: Image,
    image_size: ImageThumbType,
) -> TiberiusResult<Markup> {
    let link = image_url(either::Right(&image));
    if image.duplicate_id.is_some() {
        return Ok(html! { .media-box__overlay { strong { "Marked duplicate" } } });
    }
    if image.destroyed_content {
        return Ok(html! { .media-box__overlay { strong { "Destroyed content" } } });
    }
    if image.hidden_from_users {
        return Ok(html! { .media-box__overlay { strong {
            "Deleted: " (image.deletion_reason.unwrap_or("Unknown".to_string()))
        } } });
    }
    Ok(RenderIntent::from_image(req, client, image, image_size)
        .await?
        .render(link)?)
}

impl RenderIntent {
    pub async fn from_image(
        req: &Request<'_>,
        client: &mut Client,
        image: Image,
        size: ImageThumbType,
    ) -> TiberiusResult<Self> {
        let vid = image.image_mime_type.clone().unwrap_or_default() == "video/webm";
        let gif = image.image_mime_type.clone().unwrap_or_default() == "image/gif";
        let alt = image.title_text(client).await?;
        let hidpi = req
            .cookies()
            .get("hidpi")
            .map(|x| x.value() == "true")
            .unwrap_or(false);
        let webm = req
            .cookies().get("webm")
            .map(|x| x.value() == "true")
            .unwrap_or(false);
        let use_gif = vid
            && !webm
            && match size {
                ImageThumbType::Thumb => true,
                ImageThumbType::ThumbSmall => true,
                ImageThumbType::ThumbTiny => true,
                _ => false,
            };
        let filtered = image.filter_or_spoiler_hits(client).await?;
        Ok(if filtered && vid {
            RenderIntent::FilteredVideo { hover_text: alt }
        } else if filtered && !vid {
            RenderIntent::FilteredImage { hover_text: alt }
        } else if hidpi && !(gif || vid) {
            let small_url: PathBuf = thumb_url(req, client, either::Right(&image), ImageThumbType::Small)
                .await?;
            let small_url: Cow<str> = small_url
                .to_string_lossy();
            let small_url: String = small_url.to_string();
            let medium_url: PathBuf = thumb_url(req, client, either::Right(&image), ImageThumbType::Medium)
                .await?;
            let medium_url = medium_url
                .to_string_lossy()
                .to_string();
            RenderIntent::HiDpi {
                small_url,
                medium_url,
                hover_text: alt,
            }
        } else if !vid || use_gif {
            let small_url: PathBuf = thumb_url(req, client, either::Right(&image), ImageThumbType::Small)
                .await?;
            let small_url: String = small_url
                .to_string_lossy()
                .to_string();
                let medium_url: PathBuf = thumb_url(req, client, either::Right(&image), ImageThumbType::Medium)
                .await?;
            let medium_url = medium_url
                .to_string_lossy()
                .to_string();
            RenderIntent::Image {
                small_url,
                medium_url,
                hover_text: alt,
                webm,
            }
        } else {
            let path: PathBuf = thumb_url(req, client, either::Right(&image), size).await?;
            RenderIntent::Video {
                webm: path.to_string_lossy().to_string(),
                mp4: path.to_string_lossy().replace(".webm", ".mp4"),
                hover_text: alt,
            }
        })
    }
    pub fn render(self, link: PathBuf) -> TiberiusResult<Markup> {
        use RenderIntent::*;
        Ok(match self {
            FilteredImage { hover_text } => {
                html! {
                    .media-box__overlay.js-spoiler-info-overlay {}
                    a href=(link.to_string_lossy()) title=(hover_text) {
                        picture {
                            img alt=(hover_text) {}
                        }
                    }
                }
            }
            Image {
                small_url,
                medium_url: _,
                hover_text,
                webm,
            } => {
                html! {
                    .media-box__overlay-js-spoiler-info-overlay {
                        @if webm {
                            "WebM"
                        }
                    }
                    a href=(link.to_string_lossy()) title=(hover_text) {
                        picture {
                            img src=(small_url) alt=(hover_text);
                        }
                    }
                }
            }
            v => todo!(" other image features like {:?}", v),
        })
    }
}
