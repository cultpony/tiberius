use axum::extract::State;
use axum::{Extension, Router};
use axum_extra::routing::{TypedPath, RouterExt};
use maud::html;
use serde::Deserialize;
use tiberius_core::app::PageTitle;
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::{TiberiusResponse, HtmlResponse};
use tiberius_core::session::Unauthenticated;
use tiberius_core::state::{TiberiusRequestState, TiberiusState};

use crate::pages::common::filters::filter_listing_item;

pub fn setup_filters(r: Router<TiberiusState>) -> Router<TiberiusState> {
    r.typed_get(index)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/filters")]
pub struct PathFilters;

#[instrument(skip(state, rstate))]
pub async fn index(
    _: PathFilters,
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    //TODO: set image title correctly
    let mut client = state.get_db_client();
    let user = rstate.user(&state).await?;
    let body = html!{
        .walloftext {
            .block.block--fixed.block--warning {
                h2 { "Content Safety" }
                p {
                    r#"By default, content that is safe and suitable for all ages is all you'll see on the site, and how most of our users browse. The default filters focus on art, so filter out things like memes and what some users would consider "spam"."#
                }
                p {
                    r#"Filters let you customize what content you see on the site. This means that with the appropriate filter selected, you can access content which is not suitable for everyone, such as sexually explicit, grimdark or gory material."#
                }
                p {
                    strong {
                        r#"By changing away from the default filters, you accept you are legally permitted to view this content in your jurisdiction. If in doubt, stick with the recommended default filters."#
                    }
                }
            }
            h1 {
                "Browsing Filters"
            }
            p {
                r#"Images posted on the site are tagged, allowing you to easily search for content. You can also filter out content you'd rather not see using filters. Filters are sets of tags - spoilered tags and hidden tags. Spoilers are images that show up as thumbnails instead of the image, letting you click through and find out more about an image before deciding to view it or not. Hidden tags will simply hide images."#
                " "
            }
            p {
                r#"There are set of global filters to pick from which cover some common use-cases."#
                " "
                r#"If you're logged in you can also customize these filters and make your own, as well as quickly switch (via the menu on every page) between them."#
            }
            h2 {
                "So how do these work?"
            }
            p {
                "You can select any filter you can see. This will become your "
                strong { "active filter" }
                r#" and will affect how you see the site.  You can edit filters if you own them - you can create a filter from scratch with the link under "My Filters" (if you're logged in, of course) or by clicking "Customize", which will copy an existing filter for you to edit."#
            }
            p {
                r#"By default all the filters you create are private and only visible by you. You can have as many as you like and switch between them instantly with no limits. You can also create a public filter, which can be seen and used by any user on the site, allowing you to share useful filters with others."#
            }
            h2 {
                "My Filters"
            }
            @if let Some(user) = user.as_ref() {
                p {
                    a href="//TODO: route new filter setup" { "Click here to make a new filter from scratch" }
                }
                @for filter in user.get_all_user_filters(&mut client).await? {
                    (filter_listing_item(&filter, &state, Some(user)).await?)
                }
            } @else {
                p { 
                    "If you're logged in, you can create and maintain custom filters here"
                }
            }
            h2 {
                "Global Filters"
            }
            @for filter in state.system_filters().await? {
                (filter_listing_item(&filter, &state, user.as_ref()).await?)
            }
        }
    };
    let app = crate::pages::common::frontmatter::app(
        &state,
        &rstate,
        Some(PageTitle::from("Filters")),
        &mut client,
        body,
        None,
    )
    .await?;
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: app.into_string(),
    }))
}

pub struct FormSetSessionFilter {
    /// The ID of the filter to use
    filter_id: u64,
    /// If set, the filter is only effective for the session
    session_only: bool,
}

#[instrument(skip(state, rstate))]
pub async fn set_filter(
    _: PathFilters,
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    todo!()
}