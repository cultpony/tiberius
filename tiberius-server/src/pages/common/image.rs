use std::{borrow::Cow, path::PathBuf};

use maud::{html, Markup, Render};
use rocket::Request;
use tiberius_core::error::TiberiusResult;
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Client, Image, ImageThumbType, ImageThumbUrl, User};

use crate::pages::common::{
    pagination::PaginationCtl,
    routes::{image_url, thumb_url},
};

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
pub enum ImageBlockHeader {
    Small,
    Default,
}

impl ToString for ImageBlockHeader {
    fn to_string(&self) -> String {
        match self {
            Self::Small => "media-box__header--small".to_string(),
            Self::Default => "".to_string(),
        }
    }
}

pub async fn image_block_default_sort<
    S: Into<String>,
    S1: Into<String>,
    S5: Into<String>,
    S6: Into<String>,
>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
    client: &mut Client,
    header: ImageBlockHeader,
    query: S,
    aquery: Vec<S5>,
    anquery: Vec<S6>,
    page: u64,
    page_size: u64,
    filter_title: S1,
) -> TiberiusResult<Markup> {
    image_block::<S, &str, &str, S1, S5, S6>(
        state,
        rstate,
        client,
        header,
        query,
        aquery,
        anquery,
        None,
        None,
        page,
        page_size,
        filter_title,
    )
    .await
}

pub async fn image_block<
    S1: Into<String>,
    S2: Into<String>,
    S3: Into<String>,
    S4: Into<String>,
    S5: Into<String>,
    S6: Into<String>,
>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
    client: &mut Client,
    header: ImageBlockHeader,
    query: S1,
    aquery: Vec<S5>,
    anquery: Vec<S6>,
    sort_by: Option<S2>,
    order_by: Option<S3>,
    page: u64,
    page_size: u64,
    filter_title: S4,
) -> TiberiusResult<Markup> {
    let (total, images) = Image::search(
        client, query, aquery, anquery, sort_by, order_by, page, page_size,
    )
    .await?;
    let pagination = PaginationCtl::new(
        0,
        25,
        &["q", "sf", "sd"],
        total,
        "images",
        "image",
        filter_title,
    )?;
    assert!(
        total as usize == images.len(),
        "image index out of step with database, index had {} images, got {}",
        total,
        images.len()
    );
    Ok(html! {
        .block #imagelist-container {
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
                    (image_box(state, rstate, client, image, ImageSize::Medium, HeaderSize::ThumbSmall).await?)
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

pub async fn image_thumb_urls(image: &Image) -> TiberiusResult<ImageThumbUrl> {
    Ok(ImageThumbUrl {
        full: PathBuf::from(
            uri!(crate::pages::files::image_full_get(
                id = image.id as u64,
                _filename = image.filename()
            ))
            .to_string(),
        ),
        large: PathBuf::from(
            uri!(crate::pages::files::image_thumb_get_simple(
                id = image.id as u64,
                thumbtype = "full",
                _filename = image.filetypef("full")
            ))
            .to_string(),
        ),
        rendered: PathBuf::from(
            uri!(crate::pages::files::image_thumb_get_simple(
                id = image.id as u64,
                thumbtype = "rendered",
                _filename = image.filetypef("rendered")
            ))
            .to_string(),
        ),
        tall: PathBuf::from(
            uri!(crate::pages::files::image_thumb_get_simple(
                id = image.id as u64,
                thumbtype = "tall",
                _filename = image.filetypef("tall")
            ))
            .to_string(),
        ),
        medium: PathBuf::from(
            uri!(crate::pages::files::image_thumb_get_simple(
                id = image.id as u64,
                thumbtype = "medium",
                _filename = image.filetypef("medium")
            ))
            .to_string(),
        ),
        small: PathBuf::from(
            uri!(crate::pages::files::image_thumb_get_simple(
                id = image.id as u64,
                thumbtype = "small",
                _filename = image.filetypef("small")
            ))
            .to_string(),
        ),
        thumb: PathBuf::from(
            uri!(crate::pages::files::image_thumb_get_simple(
                id = image.id as u64,
                thumbtype = "thumb",
                _filename = image.filetypef("thumb")
            ))
            .to_string(),
        ),
        thumb_small: PathBuf::from(
            uri!(crate::pages::files::image_thumb_get_simple(
                id = image.id as u64,
                thumbtype = "thumb_small",
                _filename = image.filetypef("thumb_small")
            ))
            .to_string(),
        ),
        thumb_tiny: PathBuf::from(
            uri!(crate::pages::files::image_thumb_get_simple(
                id = image.id as u64,
                thumbtype = "thumb_tiny",
                _filename = image.filetypef("thumb_tiny")
            ))
            .to_string(),
        ),
    })
}

pub async fn show_vote_counts(state: &TiberiusState, rstate: &TiberiusRequestState<'_>) -> bool {
    //TODO: read setting from site + user
    true
}

pub async fn uploader(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
    image: &Image,
) -> TiberiusResult<Markup> {
    Ok(html! {
        span.image_uploader {
            " by "
            "anon?"
        }
    })
}

pub async fn user_attribution(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
    user: Option<&User>,
) -> TiberiusResult<Markup> {
    Ok(html! {
        @if let Some(user) = user {
            strong {
                a href="TODO://User link" { (user.name) }
            }
            //TODO: show user and awards/badges
        }
    })
}

pub async fn image_box<'a>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
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
    let interactions = html! {
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
    debug!("showing image {} to page", image.id);
    Ok(html! {
        .media-box data-image-id=(image.id) {
            .media-box__header.media-box__header--link-row.(header_class) data-image-id=(image.id) {
                (interactions)
            }
            .media-box__content.flex.flex--centered.flex--centered-distributed.(size_class) {
                (image_container(state, rstate, client, image, image_size.into()).await?)
            }
        }
    })
}

pub async fn image_container<'a>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
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
    Ok(
        RenderIntent::from_image(state, rstate, client, image, image_size)
            .await?
            .render(link)?,
    )
}

impl RenderIntent {
    pub async fn from_image(
        state: &TiberiusState,
        rstate: &TiberiusRequestState<'_>,
        client: &mut Client,
        image: Image,
        size: ImageThumbType,
    ) -> TiberiusResult<Self> {
        let vid = image.image_mime_type.clone().unwrap_or_default() == "video/webm";
        let gif = image.image_mime_type.clone().unwrap_or_default() == "image/gif";
        let alt = image.title_text(client).await?;
        let hidpi = rstate
            .cookie_jar
            .get("hidpi")
            .map(|x| x.value() == "true")
            .unwrap_or(false);
        let webm = rstate
            .cookie_jar
            .get("webm")
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
            let small_url: PathBuf = thumb_url(
                state,
                rstate,
                client,
                either::Right(&image),
                ImageThumbType::Small,
            )
            .await?;
            let small_url: Cow<str> = small_url.to_string_lossy();
            let small_url: String = small_url.to_string();
            let medium_url: PathBuf = thumb_url(
                state,
                rstate,
                client,
                either::Right(&image),
                ImageThumbType::Medium,
            )
            .await?;
            let medium_url = medium_url.to_string_lossy().to_string();
            RenderIntent::HiDpi {
                small_url,
                medium_url,
                hover_text: alt,
            }
        } else if !vid || use_gif {
            let small_url: PathBuf = thumb_url(
                state,
                rstate,
                client,
                either::Right(&image),
                ImageThumbType::Small,
            )
            .await?;
            let small_url: String = small_url.to_string_lossy().to_string();
            let medium_url: PathBuf = thumb_url(
                state,
                rstate,
                client,
                either::Right(&image),
                ImageThumbType::Medium,
            )
            .await?;
            let medium_url = medium_url.to_string_lossy().to_string();
            RenderIntent::Image {
                small_url,
                medium_url,
                hover_text: alt,
                webm,
            }
        } else {
            let path: PathBuf =
                thumb_url(state, rstate, client, either::Right(&image), size).await?;
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
