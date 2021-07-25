use maud::{Markup, PreEscaped, html};
use rocket::form::Form;
use tiberius_core::app::PageTitle;
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::{HtmlResponse, RedirectResponse, TiberiusResponse};
use tiberius_core::state::{Flash, TiberiusRequestState, TiberiusState};
use tiberius_models::Image;
use rocket::response::Redirect;
use rocket::State;

use crate::pages::common::frontmatter::{image_clientside_data, quick_tag_table, tag_editor};
use crate::pages::common::human_date;
use crate::pages::common::image::{image_thumb_urls, show_vote_counts};

#[get("/<image>")]
pub async fn show_image(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_>,
    image: u64,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client = state.get_db_client().await?;
    let image = Image::get_id(&mut client, image as i64).await?;
    let image = match image {
        Some(image) => image,
        None => {
            return Ok(TiberiusResponse::Redirect(RedirectResponse {
                redirect: Flash::warning("Image not found")
                    .into_resp(Redirect::to(uri!(crate::pages::activity::index))),
            }))
        }
    };
    let image_size = human_bytes::human_bytes(image.image_size.unwrap_or(0));
    let image_meta = html! {
        .block.block__header {
            .flex.flex--wrap.image-metabar.center--layout id=(format!("image_meta_{}", image.id)) {
                .stretched-mobile-links {
                    a.js-prev href="//TODO:" title="Previous Image (j)" {
                        i.fa.fa-chevron-left {}
                    }
                    a.js-up href="//TODO:" title="Find this image in the global image list (i)" {
                        i.fa.fa-chevron-up {}
                    }
                    a.js-next href="//TODO:" title="Next image (k)" {
                        i.fa.fa-chevron-right {}
                    }
                    a.js-rand href="//TODO:" title="Random (r)" {
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
                        @if show_vote_counts(state, &rstate).await {
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
                    a href="TODO://view" rel="nofollow" title="View (tags in filename)" {
                        i.fa.fa-eye { " View" }
                    }
                    a href="TODO://vs" rel="nofollow" title="View (no tags in filename)" {
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
            .image-metabar.flex.flex--wrap.block__header--user-credit.center-layout#extrameta {
                div {
                    "Uploaded "
                    (human_date(image.created_at))
                }

                (PreEscaped("&nbsp;"))

                span.image-size {
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

                span title=(image_size) { (image_size) }
            }
        }
    };
    //TODO: compute this
    let use_fullsize = true;
    let scaled_value: f32 = 1.0;
    let data_uris = image_thumb_urls(&image).await?;
    let data_uris = serde_json::to_string(&data_uris)?;
    let thumb_url = uri!(crate::pages::files::image_thumb_get_simple(id = image.id as u64, thumbtype = "full", _filename = image.filename()));
    let thumb_url = thumb_url.to_string();
    let image_target = html!{
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
    let image_target = image_clientside_data(state, &rstate, &image, image_target).await?;
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
    let description = html! {};
    let description_form = html! {};
    let tags = html! {};
    let source = html! {};
    let options = html! {};
    let comment_view = html! {};
    let comment_form = html! {};
    let comments = html! {
        h4 {
            //TODO: show ban reason
            (comment_form)
        }
        #comments data-current-url=(uri!(show_image(image = image.id as u64))) data-loaded="true" {
            (comment_view)
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
        state,
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

#[get("/images/new")]
pub async fn upload_image(state: &State<TiberiusState>, rstate: TiberiusRequestState<'_>) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client = state.get_db_client().await?;
    let user = rstate.session.get_user(&mut client).await?;
    let image_form_image = html! {
        .image-other {
            #js-image-upload-previews {
                p {
                    "Upload a file from your computer, "
                    " or provide a link to the page containing the image and click Fetch. "
                }
            }
            .field {
                input.input.js-scraper#image_image type="file" name="image[image]" {}
                // TODO: show proc errors here
            }
            .field.field--inline {
                input.input.input--wide.js-scraper#image_scraper_url type="url" name="image[scraper_url]" placeholder="Link a deviantART page, a Tumblr post, or the image directly" {}
                button.button.button--seperate-left#js-scraper-preview data-disable-with="Fetch" disabled="" title="Fetch image at the specified URL" type="button" {
                    "Fetch"
                }
            }
            .field-error-js.hidden.js-scraper {}
        }
    };
    let image_form_source = html! {
        .field {
            label for="image_source_url" { "The page you found this image on" }
            input.input.input--wide.js-image-input#image_source_url name="image[source_url]" placeholder="Source URL" type="url" {}
        }
    };
    let image_tag_form = html! {
        .field {
            label for="image[tag_input]" {
                "Describe with " strong { " 3+ " } " tags, including ratings and applicable artist tags"
            }
            (tag_editor("upload", "tag_input"))

            p { "You can mouse over tags below to view a description, and click to add. Short tag names can be used and will expand to full." }

            .block.js-tagtable data-target="[name=\"image[tag_input]\"]" {
                (quick_tag_table(state))
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
                    
                    textarea.input.input--wide.input--text.js-preview-description.js-image-input.js-toolbar-input id="description" name="image[description]" placeholder="Describe this image in plain words - this should generally be info about the image that doesn't belong in the tags or source." {}
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
                input.checkbox type="checkbox" id="anonymous" name="image[anonymous]" value="true" {} //TODO: load this from server settings
            }
        }
    };
    
    let body = html! {
        form action=(uri!(new_image())) enctype="multipart/form-data" method="post" {
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
                            a href=(uri!(crate::pages::blog::show(page = "rules"))) { " site rules " }
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
                            a href=(uri!(crate::pages::images::search_reverse_page)) {
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
        state,
        &rstate,
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

pub struct ImageUpload {

}

#[post("/image", data = "<image>")]
pub async fn new_image(image: Form<ImageUpload>) -> TiberiusResult<RedirectResponse> {
    log::debug!("got image: image");
    todo!()
}

#[get("/search")]
pub async fn search_empty() -> TiberiusResult<HtmlResponse> {
    Ok(search("".to_string(), None, None).await?)
}

#[get("/search/reverse")]
pub async fn search_reverse_page() -> TiberiusResult<HtmlResponse> {
    todo!()
}

#[get("/search?<_search>&<_order>&<_direction>")]
pub async fn search(
    _search: String,
    _order: Option<String>,
    _direction: Option<String>,
) -> TiberiusResult<HtmlResponse> {
    todo!()
}
