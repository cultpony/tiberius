use crate::pages::{
    blog::PathBlogPage,
    common::{
        image::{
            image_block, image_block_default_sort, image_box, DisplaySize, HeaderSize,
            ImageBlockHeader, ImageSize,
        },
        streambox::stream_box,
    },
    images::{PathQuerySearch, PathSearchEmpty, QuerySearch},
};
use axum::{Extension, Router, extract::State};
use axum_extra::routing::{RouterExt, TypedPath};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use tiberius_core::{
    acl::{ACLActionImage, ACLObject, ACLSubject},
    error::TiberiusResult,
    request_helper::{HtmlResponse, TiberiusResponse},
    session::{SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_models::{Client, Image, ImageSortBy, SortDirection};

pub fn activity_pages(r: Router<TiberiusState>) -> Router<TiberiusState> {
    r.typed_get(index)
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/")]
pub struct PathActivityIndex {}

#[instrument(skip(state, rstate))]
pub async fn index(
    _: PathActivityIndex,
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client: Client = state.get_db_client();
    let show_sidebar = true; //TODO: check setting
    let featured_image = Image::get_featured(&mut client).await?;
    let body = html! {
        div.column-layout {
            @if show_sidebar {
                aside.column-layout__left #activity-side {
                    @if let Some(featured_image) = featured_image {
                        @if !featured_image.hidden(&mut client)? {
                            .center {
                                h4.remove-top-margin { "Manebooru Spotlight" }
                                (image_box(&state, &rstate, &mut client, featured_image, ImageSize::Medium, HeaderSize::None, DisplaySize::Featured).await?)
                            }
                        }
                    }
                    .block.block--fixed.block--fixed--sub.block--success.center.hide-mobile {
                        "Enjoy the site? "
                        a href=(PathBlogPage{page: "donations".to_string()}.to_uri().to_string()) { "Donate to keep it going!" }
                    }
                    .block.block--fixed.block--fixed--sub.center.hide-mobile {
                        "Issues? Want to chat? "
                        a href=(PathBlogPage{page: "context".to_string()}.to_uri().to_string()) { "Contact us!" }
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href=(PathQuerySearch{search: "created_at.gte:10 minutes ago".to_string(), order: Some("wilson_score".to_string()), direction: Some("desc".to_string())}.to_uri()?.to_string()) {
                            "Trending Images"
                        }
                        .block__content.flex.flex--centered.flex--wrap.image-flex-grid {
                            @for image in Image::search(&mut client, "created_at.gte:10 minutes ago", vec!["safe", "processed.eq:true"], vec!["deleted.eq:true"], ImageSortBy::WilsonScore(SortDirection::Descending), 0, 4).await?.1 {
                                (image_box(&state, &rstate, &mut client, image, ImageSize::Medium, HeaderSize::ThumbSmall, DisplaySize::Normal).await?)
                            }
                        }
                        a.block__header--single-item.center href=(PathQuerySearch{search: "".to_string(), order: Some("score".to_string()), direction: Some("desc".to_string())}.to_uri()?.to_string()) { "All Time Top Scoring" }
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
                        a.block__header--single-item.center href=(PathQuerySearch{search: "created_at.gte:10 minutes ago".to_string(), order: Some("comment_count".to_string()), direction: Some("desc".to_string())}.to_uri()?.to_string()) {
                            "Most Commented-on Images"
                        }
                    }
                }
                .column-layout__main {
                    (image_block_default_sort(&state, &rstate, &mut client, ImageBlockHeader::Default, "created_at.lte:10 minutes ago", vec!["safe", "processed.eq:true"], vec!["deleted.eq:true"], 0, 25, "recently uploaded").await?)
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
