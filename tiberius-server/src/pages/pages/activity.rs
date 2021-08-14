use crate::pages::common::{
    image::{
        image_block, image_block_default_sort, image_box, HeaderSize, ImageBlockHeader, ImageSize,
    },
    streambox::stream_box,
};
use maud::{html, Markup, PreEscaped};
use rocket::{Request, State};
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::{HtmlResponse, TiberiusResponse};
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Client, Image};

#[get("/")]
pub async fn index(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let state = state.inner().clone();
    let mut client: Client = state.get_db_client().await?;
    let show_sidebar = true; //TODO: check setting
    let featured_image = Image::get_featured(&mut client).await?;
    let body = html! {
        div.column-layout {
            @if show_sidebar {
                aside.column-layout__left#activity-side {
                    @if let Some(featured_image) = featured_image {
                        @if !featured_image.hidden(&mut client)? {
                            .center {
                                h4.remove-top-margin { "Featured Image" }
                                (image_box(&state, &rstate, &mut client, featured_image, ImageSize::Medium, HeaderSize::None).await?)
                            }
                        }
                    }
                    .block.block--fixed.block--fixed--sub.block--success.center.hide-mobile {
                        "Enjoy the site?"
                        a href=(uri!(crate::pages::blog::show(page = "donations"))) { "Donate to keep it going!" }
                    }
                    .block.block--fixed.block--fixed--sub.center.hide-mobile {
                        "Issues? Want to chat?"
                        a href=(uri!(crate::pages::blog::show(page = "contact"))) { "Contact us!" }
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href=(uri!(crate::pages::pages::images::search(_search="created_at.gte:10 minutes ago", _order=Some("wilson_score"), _direction=Some("desc")))) {
                            "Trending Images"
                        }
                        .block__content.flex.flex--centered.flex--wrap.image-flex-grid {
                            @for image in Image::search(&mut client, "created_at.gte:10 minutes ago", vec!["processed.eq:true"], vec!["deleted.eq:true"], Some("wilson_score"), Some("desc"), 0, 4).await?.1 {
                                (image_box(&state, &rstate, &mut client, image, ImageSize::Medium, HeaderSize::ThumbSmall).await?)
                            }
                        }
                        a.block__header--single-item.center href=(uri!(crate::pages::pages::images::search(_search="", _order=Some("score"), _direction=Some("desc")))) { "All Time Top Scoring" }
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href="/channels" { "Streams" }
                        (stream_box(&rstate, &mut client).await?)
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href="/forums" { "Forum Activity" }
                        //TODO: implement forum activity box
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href="/comments" { "Recent Comments" }
                        //TODO: show recent comments
                        a.block__header--single-item.center href=(uri!(crate::pages::pages::images::search(_search="created_at.lte:10 minutes ago", _order=Some("comment_count"), _direction=Some("desc")))) {
                            "Most Commented-on Images"
                        }
                    }
                }
                .column-layout__main {
                    (image_block_default_sort(&state, &rstate, &mut client, ImageBlockHeader::Default, "created_at.lte:10 minutes ago", vec!["processed.eq:true"], vec!["deleted.eq:true"], 0, 25, "recently uploaded").await?)
                }
            }
        }
    };
    let page: PreEscaped<String> = html! {
        (crate::pages::common::frontmatter::app(&state, &rstate, None, &mut client, body, None).await?);
    };
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: page.into_string(),
    }))
}
