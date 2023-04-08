use std::{borrow::Cow, fmt::Display, path::PathBuf};

use axum::http::Uri;
use axum_extra::routing::TypedPath;
use itertools::Itertools;
use maud::{html, Markup, Render};
use tiberius_core::{
    error::TiberiusResult,
    session::{SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_dependencies::chrono::Datelike;
use tiberius_models::{
    Client, Image, ImageSortBy, ImageThumbType, ImageThumbUrl, PathImageThumbGet, SortDirection,
    User,
};

use crate::pages::{
    common::{frontmatter::CSSWidth, pagination::PaginationCtl},
    images::PathShowImage,
    PathImageThumbGetSimple,
};

#[derive(Debug, Clone, Copy)]
pub enum ImageSize {
    Large,
    Medium,
    Small,
}

impl From<ImageSize> for ImageThumbType {
    fn from(val: ImageSize) -> Self {
        use ImageSize::*;
        match val {
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
    S: Into<String> + std::fmt::Debug,
    S1: Into<String> + std::fmt::Debug,
    S5: Into<String> + std::fmt::Debug,
    S6: Into<String> + std::fmt::Debug,
>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<Unauthenticated>,
    client: &mut Client,
    header: ImageBlockHeader,
    query: S,
    aquery: Vec<S5>,
    anquery: Vec<S6>,
    page: u64,
    page_size: u64,
    filter_title: S1,
) -> TiberiusResult<Markup> {
    image_block::<S, S1, S5, S6>(
        state,
        rstate,
        client,
        header,
        query,
        aquery,
        anquery,
        ImageSortBy::CreatedAt(SortDirection::Descending),
        page,
        page_size,
        filter_title,
    )
    .await
}

pub async fn image_block<
    S1: Into<String> + std::fmt::Debug,
    S4: Into<String> + std::fmt::Debug,
    S5: Into<String> + std::fmt::Debug,
    S6: Into<String> + std::fmt::Debug,
>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<Unauthenticated>,
    client: &mut Client,
    header: ImageBlockHeader,
    query: S1,
    aquery: Vec<S5>,
    anquery: Vec<S6>,
    sort_by: ImageSortBy,
    page: u64,
    page_size: u64,
    filter_title: S4,
) -> TiberiusResult<Markup> {
    let (total, mut images) =
        Image::search(client, query, aquery, anquery, sort_by, page, page_size).await?;
    images.reverse();
    debug!(
        "Got {total} images: {:?}",
        images.iter().map(|x| x.id).collect_vec()
    );
    let pagination = PaginationCtl::new(
        0,
        25,
        &["q", "sf", "sd"],
        total,
        "images",
        "image",
        filter_title,
    )?;
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
                    (image_box(state, rstate, client, image, ImageSize::Medium, HeaderSize::ThumbSmall, DisplaySize::Normal).await?)
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

pub async fn show_vote_counts(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<Unauthenticated>,
) -> bool {
    //TODO: read setting from site + user
    true
}

pub async fn uploader(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<Unauthenticated>,
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
    rstate: &TiberiusRequestState<Unauthenticated>,
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

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum DisplaySize {
    Normal,
    Featured,
}

impl DisplaySize {
    pub fn to_width(&self) -> CSSWidth {
        match self {
            DisplaySize::Normal => CSSWidth::Pixels(248),
            DisplaySize::Featured => CSSWidth::Pixels(326),
        }
    }
}

impl Display for DisplaySize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_width().to_string())
    }
}

pub async fn image_box<'a>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<Unauthenticated>,
    client: &mut Client,
    image: Image,
    image_size: ImageSize,
    header_size: HeaderSize,
    display_size: DisplaySize,
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
            " "
            span.favorites title="Favorites" data-image-id=(image.id) { (image.faves_count) }
        }
        a.interaction--upvote href="#" rel="nofollow" data-image-id=(image.id) {
            i.fa.fa-arrow-up title="Yay!" {}
            " "
            span.score title="Score" data-image-id=(image.id) { (image.score) }
        }
        a.interaction--comments href=(format!("/{}#comments", image.id)) title="Comments" {
            i.fa.fa-comments {}
            " "
            span.comments-count data-image-id=(image.id) { (image.comments_count) }
        }
        a.interaction--hide href="#" rel="nofollow" data-image-id=(image.id) {
            i.fa.fa-eye-slash title="Hide" {}
        }
    };
    debug!("showing image {} to page", image.id);
    Ok(html! {
        .media-box data-image-id=(image.id) style=(format!("width: {display_size};")) {
            .media-box__header.media-box__header--link-row.(header_class) data-image-id=(image.id) {
                (interactions)
            }
            .media-box__content.flex.flex--centered.flex--centered-distributed.(size_class) style=(format!("width: {display_size}; height: {display_size};")) {
                (image_container(state, rstate, client, image, image_size.into()).await?)
            }
        }
    })
}

pub async fn image_container<'a>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<Unauthenticated>,
    client: &mut Client,
    image: Image,
    image_size: ImageThumbType,
) -> TiberiusResult<Markup> {
    let link = PathShowImage {
        image: image.id as u64,
    }
    .to_uri();
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
    Ok(html! {
        div.image-container.thumb {
            (RenderIntent::from_image(state, rstate, client, image, image_size)
                .await?
                .render(link)?)
        }
    })
}

impl RenderIntent {
    pub async fn from_image(
        state: &TiberiusState,
        rstate: &TiberiusRequestState<Unauthenticated>,
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
        let static_host = state.config.static_host(Some(rstate));
        fn apply_static_host(uri: Uri, host: Option<&String>) -> Uri {
            let host = match host {
                None => return uri,
                Some(host) => host.clone(),
            };
            let base = Uri::try_from(host).unwrap();
            let authority = base.authority().unwrap();
            let path_and_query = uri.path_and_query().unwrap();
            Uri::builder()
                .scheme(base.scheme().unwrap().clone())
                .authority(authority.clone())
                .path_and_query(path_and_query.clone())
                .build()
                .expect("was already valid")
        }
        Ok(if filtered && vid {
            RenderIntent::FilteredVideo { hover_text: alt }
        } else if filtered && !vid {
            RenderIntent::FilteredImage { hover_text: alt }
        } else if hidpi && !(gif || vid) {
            let small_url = apply_static_host(
                Uri::builder()
                    .path_and_query(
                        PathBuf::from(
                            PathImageThumbGet {
                                id: image.id as u64,
                                filename: image.filetypef("small"),
                                year: image.created_at.year() as u16,
                                month: image.created_at.month() as u8,
                                day: image.created_at.day() as u8,
                            }
                            .to_uri()
                            .to_string(),
                        )
                        .to_string_lossy()
                        .to_string(),
                    )
                    .build()
                    .unwrap(),
                Some(&static_host),
            )
            .to_string();
            let medium_url = apply_static_host(
                Uri::builder()
                    .path_and_query(
                        PathBuf::from(
                            PathImageThumbGet {
                                id: image.id as u64,
                                filename: image.filetypef("medium"),
                                year: image.created_at.year() as u16,
                                month: image.created_at.month() as u8,
                                day: image.created_at.day() as u8,
                            }
                            .to_uri()
                            .to_string(),
                        )
                        .to_string_lossy()
                        .to_string(),
                    )
                    .build()
                    .unwrap(),
                Some(&static_host),
            )
            .to_string();
            RenderIntent::HiDpi {
                small_url,
                medium_url,
                hover_text: alt,
            }
        } else if !vid || use_gif {
            let small_url = apply_static_host(
                Uri::builder()
                    .path_and_query(
                        PathBuf::from(
                            PathImageThumbGet {
                                id: image.id as u64,
                                filename: image.filetypef("small"),
                                year: image.created_at.year() as u16,
                                month: image.created_at.month() as u8,
                                day: image.created_at.day() as u8,
                            }
                            .to_uri()
                            .to_string(),
                        )
                        .to_string_lossy()
                        .to_string(),
                    )
                    .build()
                    .unwrap(),
                Some(&static_host),
            )
            .to_string();
            let medium_url = apply_static_host(
                Uri::builder()
                    .path_and_query(
                        PathBuf::from(
                            PathImageThumbGet {
                                id: image.id as u64,
                                filename: image.filetypef("medium"),
                                year: image.created_at.year() as u16,
                                month: image.created_at.month() as u8,
                                day: image.created_at.day() as u8,
                            }
                            .to_uri()
                            .to_string(),
                        )
                        .to_string_lossy()
                        .to_string(),
                    )
                    .build()
                    .unwrap(),
                Some(&static_host),
            )
            .to_string();
            RenderIntent::Image {
                small_url,
                medium_url,
                hover_text: alt,
                webm,
            }
        } else {
            let path = apply_static_host(
                Uri::builder()
                    .path_and_query(
                        PathBuf::from(
                            PathImageThumbGet {
                                id: image.id as u64,
                                filename: image.filetypef(size.to_string()),
                                year: image.created_at.year() as u16,
                                month: image.created_at.month() as u8,
                                day: image.created_at.day() as u8,
                            }
                            .to_uri()
                            .to_string(),
                        )
                        .to_string_lossy()
                        .to_string(),
                    )
                    .build()
                    .unwrap(),
                Some(&static_host),
            )
            .to_string();
            let mp4_path = path.replacen(".webm", ".mp4", 1);
            let webm_path = path;
            RenderIntent::Video {
                webm: webm_path,
                mp4: mp4_path,
                hover_text: alt,
            }
        })
    }
    pub fn render<S: ToString>(self, link: S) -> TiberiusResult<Markup> {
        use RenderIntent::*;
        Ok(match self {
            FilteredImage { hover_text } => {
                html! {
                    .media-box__overlay.js-spoiler-info-overlay {}
                    a href=(link.to_string()) title=(hover_text) {
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
                    a href=(link.to_string()) title=(hover_text) {
                        picture {
                            img src=(small_url) alt=(hover_text);
                        }
                    }
                }
            }
            v => html! {}, //todo!(" other image features like {:?}", v),
        })
    }
}
