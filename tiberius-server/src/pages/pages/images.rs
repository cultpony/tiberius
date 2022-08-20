use std::{io::Seek, str::FromStr};

use async_std::path::PathBuf;
use async_trait::async_trait;
use axum::{
    body::HttpBody,
    extract::{ContentLengthLimit, FromRequest, Multipart, Query, RequestParts},
    http::Uri,
    Extension, Router,
};
use axum_extra::routing::{RouterExt, TypedPath};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use maud::{html, Markup, PreEscaped};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tempfile::NamedTempFile;
use tiberius_core::{
    acl::*,
    app::PageTitle,
    error::{TiberiusError, TiberiusResult},
    path_and_query,
    request_helper::{HtmlResponse, RedirectResponse, TiberiusResponse},
    session::{Authenticated, SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
    PathQuery,
};
use tiberius_dependencies::{axum_flash::Flash, mime, sentry};
use tiberius_models::{comment::Comment, Image, ImageMeta};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    task::spawn_blocking,
};
use tracing::{debug, Instrument};

use crate::{
    pages::{
        activity::PathActivityIndex,
        common::{
            comment::{comment_form, comment_view, single_comment},
            frontmatter::{image_clientside_data, quick_tag_table, tag_editor},
            human_date,
            image::{image_thumb_urls, show_vote_counts},
            renderer::{textile::render_textile, textile_extensions},
            tag::tag_markup,
        },
        tags::{PathTagsByNameShowTag, PathTagsShowTag},
        PathImageGetFull, PathImageGetShort, PathImageThumbGetSimple,
    },
    set_scope_tx, set_scope_user, MAX_IMAGE_DIMENSION,
};
use axum::{response::Redirect, Form};

pub fn image_pages(r: Router) -> Router {
    r.typed_get(show_image)
        .typed_get(beta_show_image)
        .typed_get(specific_show_image)
        .typed_get(get_image_comment)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/images/random")]
pub struct PathRandomImage;

#[derive(TypedPath, Deserialize)]
#[typed_path("/images/:image/related")]
pub struct PathRelatedImage {
    image: u64,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/images/:image/navigate")]
pub struct PathNavigateImage {
    image: u64,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct QueryNavigateImage {
    rel: NavigateRelation,
    #[serde(flatten)]
    search_query: Option<QuerySearchQuery>,
}

impl PathQuery for QueryNavigateImage {}

#[derive(serde::Deserialize, serde::Serialize)]
pub enum NavigateRelation {
    #[serde(rename = "next")]
    Next,
    #[serde(rename = "find")]
    Find,
    #[serde(rename = "prev")]
    Prev,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/embed/:image/:flag")]
pub struct PathEmbedImage {
    image: u64,
    flag: String,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/embed/:image")]
pub struct PathEmbedImageNoFlag {
    image: u64,
}

pub async fn embed_image_no_flag(_: PathEmbedImageNoFlag) -> TiberiusResult<()> {
    todo!()
}

pub async fn embed_image(_: PathEmbedImage) -> TiberiusResult<()> {
    todo!()
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/beta/:image")]
pub struct PathBetaShowImage {
    pub image: u64,
}

pub async fn beta_show_image(
    PathBetaShowImage { image }: PathBetaShowImage,
    query_search: Query<QuerySearchQuery>,
    Extension(state): Extension<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    show_image(
        PathShowImage { image },
        query_search,
        Extension(state),
        rstate,
    )
    .await
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/image/:image")]
pub struct PathShowImageSpecific {
    pub image: u64,
}

pub async fn specific_show_image(
    PathShowImageSpecific { image }: PathShowImageSpecific,
    query_search: Query<QuerySearchQuery>,
    Extension(state): Extension<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    show_image(
        PathShowImage { image },
        query_search,
        Extension(state),
        rstate,
    )
    .await
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/:image")]
pub struct PathShowImage {
    pub image: u64,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct QuerySearchQuery {
    #[serde(rename = "q")]
    pub query: Option<String>,
}

impl PathQuery for QuerySearchQuery {}

pub async fn show_image(
    PathShowImage { image }: PathShowImage,
    Query(query_search): Query<QuerySearchQuery>,
    Extension(state): Extension<TiberiusState>,
    mut rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    set_scope_tx!("GET /:image");
    set_scope_user!(rstate.session().raw_user().map(|x| sentry::User {
        id: Some(x.to_string()),
        ..Default::default()
    }));
    let mut client = state.get_db_client();
    let image = Image::get_id(&mut client, image as i64).await?;
    let mut image = match image {
        Some(image) => image,
        None => {
            rstate.flash_mut().warning("Image not found");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathActivityIndex {}.to_uri().to_string().as_str(),
            )));
        }
    };
    let allow_merge_duplicate: bool = verify_acl(
        &state,
        &rstate,
        ACLObject::Image,
        ACLActionImage::MergeDuplicate,
    )
    .await?;
    let allow_count_view: bool = verify_acl(
        &state,
        &rstate,
        ACLObject::Image,
        ACLActionImage::IncrementView,
    )
    .await?;
    if allow_count_view {
        image.increment_views(&mut client).await?;
    }
    let image_meta = image.metadata(&mut client).await?;
    let image_size = human_bytes::human_bytes(image.image_size.unwrap_or(0));
    let image_meta = html! {
        .block.block__header {
            .flex.flex--wrap.image-metabar.center--layout id=(format!("image_meta_{}", image.id)) {
                .stretched-mobile-links {
                    a.js-prev href=(path_and_query(PathNavigateImage{image: image.id as u64}, Some(&QueryNavigateImage{ rel: NavigateRelation::Prev, search_query: Some(query_search.clone())}))?) title="Previous Image (j)" {
                        i.fa.fa-chevron-left {}
                    }
                    a.js-up href=(path_and_query(PathNavigateImage{image: image.id as u64}, Some(&QueryNavigateImage{ rel: NavigateRelation::Find, search_query: Some(query_search.clone())}))?) title="Find this image in the global image list (i)" {
                        i.fa.fa-chevron-up {}
                    }
                    a.js-next href=(path_and_query(PathNavigateImage{image: image.id as u64}, Some(&QueryNavigateImage{ rel: NavigateRelation::Next, search_query: Some(query_search.clone())}))?) title="Next image (k)" {
                        i.fa.fa-chevron-right {}
                    }
                    a.js-rand href=(path_and_query(PathRandomImage{}, Some(&query_search))?) title="Random (r)" {
                        i.fa.fa-random {}
                    }
                }
                .stretched-mobile-links {
                    a.interaction--fave href="#" rel="nofollow" data-image-id=(image.id) {
                        span.favorites title="Favorites" data-image-id=(image.id) {
                            (image.faves_count) " "
                        }
                        span.fave-span title="Fave!" {
                            i.fa.fa-star {}
                        }
                    }
                    a.interaction--upvote href="#" rel="nofollow" data-image-id=(image.id) {
                        @if show_vote_counts(&state, &rstate).await {
                            span.upvotes title="Upvotes" data-image-id=(image.id) { (image.upvotes_count) " " }
                        }
                        span.upvote-span title="Yay!" {
                            i.fa.fa-arrow-up {}
                        }
                    }
                    a.interaction--comments href="#comments" title="Comments" {
                        i.fa.fa-comments {}
                        span.comments_count data-image-id=(image.id) { " " (image.comments_count) }
                    }

                    span title="Views" style="padding-left: 12px; padding-right: 12px;" {
                        i.fa.fa-eye {}
                        span.views_count data-image-id=(image.id) { " " (ImageMeta::views_to_text(image_meta)) }
                    }

                    a.interaction--hide href="#" rel="nofollow" data-image-id=(image.id) {
                        span.hide-span title="Hide" {
                            i.fa.fa-eye-slash {}
                        }
                    }
                }
                .stretched-mobile-links {
                    // subscriptions
                    // add_to_gallery
                    a href="TODO://releated" title="Related Images" {
                        i.fa.fa-sitemap {
                            span.hide-limited-desktop.hide-mobile { " Related" }
                        }
                    }
                }
                .stretched-mobile-links {
                    a href=(PathImageGetFull::from_image(&mut image, &mut client).await?.to_uri()) rel="nofollow" title="View (tags in filename)" {
                        i.fa.fa-eye { " View" }
                    }
                    a href=(PathImageGetShort::from(&image)) rel="nofollow" title="View (no tags in filename)" {
                        i.fa.fa-eye { " VS" }
                    }
                    a href="TODO://download" rel="nofollow" title="Download (tags in filename)" {
                        i.fa.fa-eye { " Download" }
                    }
                    a href="TODO://dl" rel="nofollow" title="Download (no tags in filename)" {
                        i.fa.fa-eye { " DS" }
                    }
                }
            }
            .image-metabar.flex.flex--wrap.block__header--user-credit.center-layout #extrameta {
                div title=(DateTime::<Utc>::from_utc(image.created_at, Utc).to_rfc3339()) {
                    "Uploaded "
                    (human_date(image.created_at))
                }

                (PreEscaped("&nbsp;"))

                span.image-size title=(format!("{} pixels", image.image_width.unwrap_or(0) * image.image_height.unwrap_or(0))) {
                    (PreEscaped("&nbsp;"))
                    (image.image_width.unwrap_or(0))
                    "x"
                    (image.image_height.unwrap_or(0))
                }

                (PreEscaped("&nbsp;"))

                @if let Some(image_duration) = image.image_duration {
                    @if image.is_animated && image_duration > 0.0 {
                        //TODO: get animation length
                    }
                }

                (PreEscaped("&nbsp;"))

                (image.image_format.as_ref().map(|x| x.to_ascii_uppercase()).unwrap_or("???".to_string()))

                (PreEscaped("&nbsp;"))

                span title=(format!("{} bytes", image.image_size.unwrap_or(0))) { (image_size) }

                (PreEscaped("&nbsp;"))
                // TODO: put this into the CSS
                //span style="margin-left: 1em;" title="This is a rough estimation of how many times this image was shown" { b { (ImageMeta::views_to_text(image_meta)) } }
            }
        }
    };
    //TODO: compute this
    let use_fullsize = true;
    let scaled_value: f32 = 1.0;
    let data_uris = image_thumb_urls(&image)
        .await?
        .with_host(Some(state.config().static_host(Some(&rstate))));
    let data_uris = serde_json::to_string(&data_uris)?;
    let thumb_url = PathImageThumbGetSimple {
        id: image.id as u64,
        thumbtype: "full".to_string(),
        filename: image.filename(),
    };
    let thumb_url = thumb_url.to_uri().to_string();
    let image_target = html! {
        .block.block--fixed.block--warning.block--no-margin.image-filtered.hidden {
            strong {
                a href="#" { "This image is blocked by your current filter - click here to display it anyway" }
            }
            p {
                //TODO: add image blocked svg
            }
        }
        @if use_fullsize {
            #image_target.hidden.image-show data-scaled=(scaled_value) data-uris=(data_uris) data-width=(image.image_width.unwrap_or(0)) data-height=(image.image_height.unwrap_or(0)) data-image-size=(image.image_size.unwrap_or(0)) data-mime-type=(image.image_mime_type.clone().unwrap_or("image/png".to_string())) {
                @if image.image_mime_type == Some("video/webm".to_string()) {
                    video controls="true" {}
                } @else {
                    picture {}
                }
            }
        } @else {
            .image-show.hidden {
                a href="//TODO: raw image" title=(image.title_text(&mut client).await?) {
                    span.imgspoiler {
                        @if image.image_mime_type == Some("video/webm".to_string()) {
                            video data-image-id=(image.id) autoplay="autoplay" loop="loop" muted="muted" playsinline="playsinline" {
                                source src=(thumb_url) type="video/webm";
                                source src=(thumb_url.replace(".webm", ".mp4")) type="video/mp4";
                            }
                        } @else {
                            picture data-image-id=(image.id) {
                                img src=(thumb_url);
                            }
                        }
                    }
                }
            }
        }
    };
    let image_target = image_clientside_data(&state, &rstate, &image, image_target).await?;
    let image_page = html! {
        .center--layout--flex {
            @if image.thumbnails_generated {
                (image_target)
            } @else {
                #thumbnails-not-yet-generated.block.block--fixed.block--warning.layout--narrow {
                    h3 {
                        "Just a moment"
                    }
                    @if image.image_mime_type == Some("video/webm".to_string()) {
                        p { "WEBM uploads may take longer to process, it should appear in up to an hour (depending on file size and video length)." }
                    } @else {
                        p { "The image should appear in a few minutes; report it otherwise." }
                    }
                    p { "Implications might have added some tags, so check everything applies." }
                    p { "If you are using a default filter, new images might be filtered out for some time to allow for correction of mistagging. Do not worry, your upload will be seen." }
                }
            }
            @if !image.processed && image.thumbnails_generated {
                br;
                #image-being-optimized.block.block--fixed.block--warning.layout--narrow {
                    "This image is being processed to optimize the filesize. It should finish shortly."
                }
            }
        }
    };
    let advert_box = html! {
        // TODO: implement adverts
    };
    let description = html! {
        div {
            // todo: add description edit form
            p {
                "Description";
                .image-description__text {
                    (render_textile(&image.description))
                }
            }
        }
    };
    let description_form = html! {};
    let tag_data = image
        .get_quick_tags(&mut client)
        .await?
        .expect("no quicktag view available");
    let tag_data = tag_data.get_tags();
    let tags = html! {
        div.tagsauce {
            div.block {}
            div.tag-list {
                @for tag in tag_data.iter().sorted() {
                    (tag_markup(tag))
                }
            }
        }
    };
    let source = html! {
        .block {
            // TODO: source change form
            .flex.flex--wrap id="image-source" {
                p {
                    a.button.button--separate-right id="edit-source" data-click-focus="#source-field" data-click-hide="#image-source" data-click-show="#source-form" title="Edit source" accessKey="s" {
                        i.fas.fa-edit {
                            "Source: "
                        }
                    }
                }
                p {
                    @if let Some(source_url) = image.source_url.as_ref() {
                        a.js-source-link href=(source_url) {
                            strong { (source_url) }
                        }
                    } @else {
                        em { "not provided yet" }
                    }

                    @if image.source_change_count().await > 1 {
                        a.button.button--link.button--separate-left href=(PathChangeImageSource{image: image.id as u64}.to_uri()) title="Source history" {
                            i.fa.fa-history {
                                "History (" (image.source_change_count().await) ")"
                            }
                        }
                    }
                    // TODO: source staff tools
                }
            }
        }
    };
    let options = html! {};
    let comments = html! {
        h4 { "Comments" }
        //(comment_form(&mut client, rstate.user(&state).await?, &image).await?)
        #comments data-current-url=(PathShowImage{ image: image.id as u64}.to_uri()) data-loaded="true" {
            (comment_view(&state, &mut client, &image).await?)
        }
    };
    let body = html! {
        (image_meta)
        (image_page)
        .layout--narrow {
            (advert_box)
            .image-description {
                (description)
            }
            (description_form)
            (tags)
            (source)
            (options)
            (comments)
        }
    };
    //TODO: set image title correctly
    let app = crate::pages::common::frontmatter::app(
        &state,
        &rstate,
        Some(PageTitle::from("Image")),
        &mut client,
        body,
        Some(image),
    )
    .await?;
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: app.into_string(),
    }))
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/images/new")]
pub struct PathUploadImagePage {}

pub async fn upload_image(
    Extension(state): Extension<TiberiusState>,
    rstate: TiberiusRequestState<Authenticated>,
    _: PathUploadImagePage,
) -> TiberiusResult<TiberiusResponse<()>> {
    use tiberius_core::session::Authenticated;

    use crate::pages::blog::PathBlogPage;

    let mut client = state.get_db_client();
    let user = rstate.session().get_user(&mut client).await?;
    let image_form_image = html! {
        .image-other {
            #js-image-upload-previews {
                p {
                    "Upload a file from your computer, "
                    " or provide a link to the page containing the image and click Fetch. "
                }
            }
            .field {
                input.input.js-scraper #image_image type="file" name="image.image" {}
                // TODO: show proc errors here
            }
            .field.field--inline {
                input.input.input--wide.js-scraper #image_scraper_url type="url" name="image.scraper_url" placeholder="Link a deviantART page, a Tumblr post, or the image directly" {}
                button.button.button--seperate-left #js-scraper-preview data-disable-with="Fetch" disabled="" title="Fetch image at the specified URL" type="button" {
                    "Fetch"
                }
            }
            .field-error-js.hidden.js-scraper {}
        }
    };
    let image_form_source = html! {
        .field {
            label for="image_source_url" { "The page you found this image on" }
            input.input.input--wide.js-image-input #image_source_url name="image.source_url" placeholder="Source URL" type="url" {}
        }
    };
    let image_tag_form = html! {
        .field {
            label for="image.tag_input" {
                "Describe with " strong { " 3+ " } " tags, including ratings and applicable artist tags"
            }
            (tag_editor("upload", "tag_input"))

            p { "You can mouse over tags below to view a description, and click to add. Short tag names can be used and will expand to full." }

            .block.js-tagtable data-target="[name=\"image.tag_input\"]" {
                (quick_tag_table(&state))
            }
        }
    };
    let image_description_form = html! {
        .field {
            .block {
                .block__header.block__header--js-tabbed {
                    a.selected href="#" data-click-tab="write" { "Description" }
                    a href="#" data-click-tab="preview" { "Preview" }
                }
                .block__tab.selected data-tab="write" {
                    //TODO: help
                    //TODO: toolbar

                    textarea.input.input--wide.input--text.js-preview-description.js-image-input.js-toolbar-input id="description" name="image.description" placeholder="Describe this image in plain words - this should generally be info about the image that doesn't belong in the tags or source." {}
                }
                .block__tab.hidden data-tab="preview" {
                    "Loading preview..."
                }
            }
        }
    };
    let image_anon_form = html! {
        @if user.is_some() {
            .field {
                label for="anonymous" { "Post anonymously" }
                input.checkbox type="checkbox" id="anonymous" name="image.anonymous" value="true" {} //TODO: load this from server settings
            }
        }
    };

    let body = html! {
        form action=(PathImageUpload{}.to_uri()) enctype="multipart/form-data" method="post" {
            @match user {
                None  => {
                    p {
                        strong {
                            "Sorry, but anonymous uploading without login is disabled for legal reasons." " "
                            "Please log in to upload new content!" " "
                            "If you're logged in, you can post anonymously." " "
                        }
                    }
                },
                Some(_) => {
                    .dnp-warning {
                        h4 {
                            "Read the ";
                            a href=(PathBlogPage{ page: "rules".to_string() }.to_uri()) { " site rules " }
                            " and check our ";
                            a href="// TODO: dnp list link" { " do-not-post list" }
                        }
                        p {
                            "Don't post content the artist doesn't want here (or shared in general), "
                            strong { " including any commercial content " }
                        }
                    }

                    p {
                        strong {
                            "Please check it isn't already here with "
                            a href=(PathSearchReverse{}.to_uri()) {
                                " reverse search "
                            }
                        }
                    }
                    h4 { "Select an image" }
                    (image_form_image)
                    h4 { "About this image" }
                    (image_form_source)
                    (image_tag_form)
                    br;
                    (image_description_form)
                    (image_anon_form)
                    .actions {
                        button.button autocomplete="off" data-disable-with="Please wait..." type="submit" { "Upload" }
                    }
                },
            }
        }
    };
    let app = crate::pages::common::frontmatter::app(
        &state,
        &rstate.into(),
        Some(PageTitle::from("Image")),
        &mut client,
        body,
        None,
    )
    .await?;
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: app.into_string(),
    }))
}

#[derive(Debug)]
pub struct ImageUpload {
    pub anonymous: bool,
    pub source_url: Option<String>,
    pub tag_input: String,
    pub description: Option<String>,
    pub scraper_url: Option<String>,
    pub image: NamedTempFile,
    pub content_type: mime::Mime,
}

#[async_trait]
impl<B> FromRequest<B> for ImageUpload
where
    B: Send,
{
    type Rejection = TiberiusError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let state = req.extensions().get::<TiberiusState>().unwrap().clone();
        let limit = state.config().upload_max_size;
        let multipart = todo!();
        Ok(spool_multipart(multipart, limit).await?)
    }
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/image")]
pub struct PathImageUpload {}

#[cfg(feature = "process-images")]
pub async fn new_image(
    Extension(state): Extension<TiberiusState>,
    mut rstate: TiberiusRequestState<Authenticated>,
    _: PathImageUpload,
    image_metadata: ImageUpload,
) -> TiberiusResult<TiberiusResponse<()>> {
    use axum::Extension;
    use tempfile::PersistError;
    use tiberius_dependencies::hex;

    tracing::debug!("got image: {:?}", image_metadata);
    let image_path = image_metadata.image.path();
    let content_type = image_metadata.content_type.clone();
    debug!("Got image content_type: {:?}", content_type);
    let content_type = content_type.to_string();
    let ext = match content_type.as_str() {
        // Images
        "image/png" => ".png",
        "image/gif" => ".gif",
        "image/bmp" => ".bmp",
        "image/jpeg" => ".jpg",
        "image/webp" => ".webp",
        "image/avif" => ".avif",
        "image/svg+xml" => {
            rstate
                .flash_mut()
                .error("We don't support SVG uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        "image/x-icon" => ".ico",
        "image/tiff" => ".tiff",
        // Audio
        "audio/flac" => {
            rstate.flash_mut().error("We don't audio uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        "audio/wav" => {
            rstate.flash_mut().error("We don't audio uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        "audio/aac" => {
            rstate.flash_mut().error("We don't audio uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        "audio/webm" => {
            rstate.flash_mut().error("We don't audio uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        // Video,
        "video/ogg" => {
            rstate.flash_mut().error("We don't video uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        "video/webm" => {
            rstate.flash_mut().error("We don't video uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        "video/mpeg" => {
            rstate.flash_mut().error("We don't video uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        "video/mp4" => {
            rstate.flash_mut().error("We don't video uploads yet.");
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
        // Other
        q => {
            rstate
                .flash_mut()
                .error(format!("We can't process images of the type {}", q));
            return Ok(TiberiusResponse::Redirect(Redirect::to(
                PathUploadImagePage {}.to_uri().to_string().as_str(),
            )));
        }
    };
    {
        let img = image::io::Reader::open(image_path)?;
        match img.with_guessed_format()?.into_dimensions() {
            Ok(v) => {
                debug!("Image metadata: {:?}", v);
                if v.0 > MAX_IMAGE_DIMENSION || v.1 > MAX_IMAGE_DIMENSION {
                    rstate.flash_mut().error(format!("We can't process image: It's too large, the image is {}x{} but we only support up to {}x{}", v.0, v.1, MAX_IMAGE_DIMENSION, MAX_IMAGE_DIMENSION));
                    return Ok(TiberiusResponse::Redirect(Redirect::to(
                        PathUploadImagePage {}.to_uri().to_string().as_str(),
                    )));
                }
                debug!("Image within max dimensions, proceeding");
            }
            Err(e) => {
                rstate
                    .flash_mut()
                    .error(format!("We can't process image: {}", e));
                return Ok(TiberiusResponse::Redirect(Redirect::to(
                    PathUploadImagePage {}.to_uri().to_string().as_str(),
                )));
            }
        }
    }
    let (sha3_256_hash, sha512_hash) = {
        let mut file = std::fs::File::open(image_path)?;
        let mut hasher_sha3 = sha3::Sha3_256::default();
        let mut hasher_sha2 = sha2::Sha512::default();
        let res = spawn_blocking(move || -> Result<(String, String), std::io::Error> {
            std::io::copy(&mut file, &mut hasher_sha3)?;
            std::io::copy(&mut file, &mut hasher_sha2)?;
            let sha3 = hex::encode(hasher_sha3.finalize().to_vec());
            let sha2 = hex::encode(hasher_sha2.finalize().to_vec());
            Ok((sha3, sha2))
        })
        .await??;
        res
    };
    let config = state.config();
    let unixts = chrono::Utc::now();
    let mut subdir = config
        .data_root
        .clone()
        .expect("require configured data root directory");
    subdir.push("images");
    if !subdir.exists() {
        std::fs::create_dir(&subdir)?;
    }
    subdir.push(unixts.format("%Y/%m/%d").to_string());
    if !subdir.exists() {
        std::fs::create_dir_all(&subdir)?;
    }
    // 128 characters prevents issues with ZFS and similar filesystems with 255 max filenames
    // 128 is still enough to provide dedup over any single day unless someone uploads 2⁶⁴ files
    subdir.push(format!("{}{}", &sha3_256_hash[0..(128 / 8)], ext));
    let new_path = subdir;
    if new_path.exists() {
        //TODO: handle existing uploads instead of ignoring it
        todo!("implement handling already uploaded images");
    } else {
        debug!("persisting file to {}", new_path.display());
        todo!("persist file via image_metadata.image.persist_noclobber(&new_path)?");
    };
    // reprocess image to ensure it's not only valid but in a good base format with good compat in all devices
    {
        match content_type.as_str() {
            "image/svg+xml" => {
                let mut nfile = new_path.clone();
                nfile.set_extension("svg11");
                let inkscape = std::process::Command::new("inkscape")
                    .arg(new_path)
                    .arg("--export-plain-svg")
                    .arg("--export-type=svg")
                    .arg("--export-filename")
                    .arg(&nfile)
                    .output()?;
                if !inkscape.status.success() {
                    return Err(tiberius_core::error::TiberiusError::Other(
                        "Inkscape could not convert to SVG1.1".to_string(),
                    ));
                }
                assert!(nfile.is_file(), "inkscape must have created new file");
                todo!("downconvert svg to svg1.1");
            }
            "image/png" => {
                debug!("png needs no downconvert");
            }
            "image/jpeg" => {
                debug!("jpeg needs no downconvert");
            }
            v => {
                todo!("downconvert {}", v)
            }
        }
    }
    let tags: Vec<(String, Option<String>)> = image_metadata
        .tag_input
        .split(',')
        .map(|x| x.trim())
        .map(|x| {
            x.split_once(":")
                .map(|(x, y)| (y.to_string(), x.to_string()))
                .map(|(x, y)| (x, Some(y)))
                .unwrap_or((x.to_string(), None))
        })
        .collect();
    let mut client = state.get_db_client();
    //TODO: create missing tags automatically
    //TODO: rewrite image from scratch to discard metadata
    let tags = tiberius_models::Tag::get_many_by_name(&mut client, tags, true).await?;
    let tags = tags.into_iter().map(|x| x.id).collect();
    let canon_path = new_path.clone();
    let canon_path =
        canon_path.strip_prefix(&config.data_root.as_ref().expect("require static data root"))?;
    let canon_path = canon_path.strip_prefix("images")?;
    //image.image.persist_to(new_path);
    let image = Image {
        image: Some(canon_path.to_string_lossy().to_string()),
        image_name: image_metadata
            .image
            .path()
            .file_name()
            .map(|x| x.to_string_lossy().to_string()),
        image_mime_type: Some(image_metadata.content_type.to_string()),
        //TODO: store IP of user
        //TODO: store fingerprint of user
        anonymous: Some(image_metadata.anonymous),
        source_url: image_metadata
            .scraper_url
            .clone()
            .or(image_metadata.source_url.clone()),
        tag_ids: tags,
        description: image_metadata
            .description
            .clone()
            .unwrap_or_else(String::new),
        ..Default::default()
    };
    let image = image.insert_new(&mut client).await?;
    #[cfg(feature = "process-images")]
    {
        use tiberius_jobs::process_image::ImageProcessConfig;
        debug!("Scheduling processing of image");
        tiberius_jobs::process_image::process_image(
            &mut client,
            ImageProcessConfig {
                image_id: image.id as u64,
            },
        )
        .await?;
    }

    rstate
        .flash_mut()
        .info("We are processing your image, it might take a few minutes");
    return Ok(TiberiusResponse::Redirect(Redirect::to(
        PathShowImage {
            image: image.id as u64,
        }
        .to_uri()
        .to_string()
        .as_str(),
    )));
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/image/:image/source_changes")]
pub struct PathChangeImageSource {
    image: u64,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/search")]
pub struct PathSearchEmpty {}

pub async fn search_empty(_: PathSearchEmpty) -> TiberiusResult<HtmlResponse> {
    Ok(search(
        PathSearchEmpty {},
        Query(QuerySearch {
            search: "".to_string(),
            order: None,
            direction: None,
        }),
    )
    .await?)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/search/reverse")]
pub struct PathSearchReverse {}

pub async fn search_reverse_page(_: PathSearchReverse) -> TiberiusResult<HtmlResponse> {
    todo!()
}

#[derive(Deserialize, Serialize)]
pub struct QuerySearch {
    search: String,
    order: Option<String>,
    direction: Option<String>,
}

impl QuerySearch {
    pub fn get_qs(&self) -> TiberiusResult<String> {
        Ok(serde_qs::to_string(&self)?)
    }
}

pub struct PathQuerySearch {
    pub search: String,
    pub order: Option<String>,
    pub direction: Option<String>,
}

impl PathQuerySearch {
    pub fn to_uri(self) -> TiberiusResult<Uri> {
        let path = PathSearchEmpty {}.to_uri().path().to_string();
        let query = QuerySearch {
            search: self.search,
            order: self.order,
            direction: self.direction,
        }
        .get_qs()?;
        Ok(Uri::builder()
            .path_and_query(format!("{path}?{query}"))
            .build()?)
    }
}

pub async fn search(_: PathSearchEmpty, query: Query<QuerySearch>) -> TiberiusResult<HtmlResponse> {
    todo!()
}

/// Spools a multipart of image upload type onto the disk
///
/// If the upload exceeds the limit number of bytes, an error is returned
pub async fn spool_multipart(mut multipart: Multipart, limit: u64) -> TiberiusResult<ImageUpload> {
    let tmpfile: tempfile::NamedTempFile = tempfile::NamedTempFile::new()?;
    let mut upload: ImageUpload = ImageUpload {
        anonymous: false,
        source_url: None,
        tag_input: String::new(),
        description: None,
        scraper_url: None,
        image: tmpfile,
        content_type: mime::TEXT_PLAIN,
    };

    while let Some(mut field) = multipart.next_field().await? {
        let name = field.name().unwrap().to_string();
        match name.as_str() {
            "anonymous" => upload.anonymous = field.text().await?.parse()?,
            "source_url" => upload.source_url = Some(field.text().await?),
            "tag_input" => upload.tag_input = field.text().await?,
            "description" => upload.description = Some(field.text().await?),
            "scraper_url" => upload.scraper_url = Some(field.text().await?),
            "image" => {
                let file: &mut std::fs::File = upload.image.as_file_mut();
                // clone so we can use tokio and a buffered writer
                let file = file.try_clone()?;
                // truncate the file in case we have someone reusing fields
                file.set_len(0)?;
                let mut file = BufWriter::new(tokio::fs::File::from_std(file));
                while let Some(chunk) = field.chunk().await? {
                    // TODO: limit write size
                    file.write(&chunk).await?;
                }
                file.flush().await?;
                upload.content_type =
                    mime::Mime::from_str(field.content_type().unwrap_or("plain/text"))?;
                drop(file);
            }
            // TODO: do something about extra fields
            _ => (),
        }
    }
    Ok(upload)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/images/:image_id/comments/:comment_id")]
pub struct PathImageComment {
    image_id: i64,
    comment_id: i64,
}

pub async fn get_image_comment(
    PathImageComment {
        image_id,
        comment_id,
    }: PathImageComment,
    Extension(state): Extension<TiberiusState>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let comment = Comment::get_by_id(&mut state.get_db_client(), comment_id).await?;
    if let Some(comment) = comment {
        if comment.image_id() != Some(image_id) {
            Err(TiberiusError::ObjectNotFound(
                "Comment".to_string(),
                comment_id.to_string(),
            ))
        } else {
            Ok(TiberiusResponse::Html(
                single_comment(&state, &mut state.get_db_client(), &comment)
                    .await?
                    .into(),
            ))
        }
    } else {
        Err(TiberiusError::ObjectNotFound(
            "Comment".to_string(),
            comment_id.to_string(),
        ))
    }
}
