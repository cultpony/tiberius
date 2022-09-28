use std::fmt;

use axum::Router;
use axum::{extract::Query, response::Redirect, Extension, Form};
use axum_extra::routing::RouterExt;
use axum_extra::{extract::cookie::Cookie, routing::TypedPath};
use maud::{html, Markup};
use serde::Deserialize;
use tiberius_core::{
    app::PageTitle,
    error::TiberiusResult,
    request_helper::{
        ApiFormDataEmpty, FormMethod, HtmlResponse, RedirectResponse, TiberiusResponse,
    },
    session::{SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
};
use tiberius_dependencies::axum_flash::Flash;
use tiberius_models::{Channel, Client, Image};
use axum_extra::extract::CookieJar;

use crate::pages::common::{channels::channel_box, pagination::PaginationCtl};

pub fn channel_pages(r: Router) -> Router {
    r
     .typed_get(list_channels)
     .typed_post(set_nsfw)
}

#[derive(serde::Deserialize, Debug)]
pub struct ChannelQuery {
    cq: Option<String>,
}

impl ChannelQuery {
    pub fn cq(&self) -> String {
        self.cq.clone().unwrap_or("".to_string())
    }
}

#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/channels/nsfw")]
pub struct PathSetChannelNsfw {}

#[instrument(skip(rstate))]
pub async fn set_nsfw(
    _: PathSetChannelNsfw,
    rstate: TiberiusRequestState<Unauthenticated>,
    fd: Form<ApiFormDataEmpty>,
) -> TiberiusResult<(CookieJar, Redirect)> {
    let mut rstate = rstate;
    let fd = fd.into_afd();
    match fd.method() {
        Some(FormMethod::Create) => {
            rstate.flash_mut().error("NSFW Channels are now visible");
            let cookie_jar = rstate.cookie_jar.add(Cookie::new("chan_nsfw", "true"));
            Ok((cookie_jar, Redirect::to(PathChannelsList {}.to_uri().to_string().as_str())))
        }
        Some(FormMethod::Delete) => {
            rstate
                .flash_mut()
                .error("NSFW Channels are now no longer visible");
            let cookie_jar = rstate.cookie_jar.add(Cookie::new("chan_nsfw", "false"));
            Ok((
                cookie_jar,
                Redirect::to(PathChannelsList {}.to_uri().to_string().as_str())
            ))
        }
        _ => Ok((
            rstate.cookie_jar,
            Redirect::to(PathChannelsList {}.to_uri().to_string().as_str())
        )),
    }
}

#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/channels/read")]
pub struct PathChannelsRead {}

#[tracing::instrument]
/// POST
pub async fn read(_: PathChannelsRead) -> String {
    todo!()
}

#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/channels")]
pub struct PathChannelsList {}

#[instrument(skip(state, rstate))]
pub async fn list_channels(
    _: PathChannelsList,
    Extension(state): Extension<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
    Query(cq): Query<ChannelQuery>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client = state.get_db_client();
    let channels = Channel::get_all_channels::<String>(&mut client, None).await?;
    //TODO: honor NSFW channel setting
    let pages = PaginationCtl::new(
        0,
        25,
        &["cq"],
        Channel::count(&mut client, false).await?,
        "channels",
        "channel",
        "",
    )?;
    let show_hide_nsfw_uri = PathSetChannelNsfw {}.to_uri().to_string();
    let show_nsfw_state = rstate.cookie_jar.get("chan_nsfw").map(|x| x.value()).unwrap_or("false") == "true";
    let body = html! {
        h1 { "Livestreams" }
        form.hform {
            .field {
                input.input.hform__text #channels_cq type="text" name="cq" value=(cq.cq()) placeholder="Search channels" autocapitalize="none";
                button.hform__button.button type="submit" { "Search" }
            }
        }
        .block {
            .block__header.page__header {
                @if pages.need_pagination() {
                    .page__pagination {
                        (pages.pagination())
                    }
                }

                @if show_nsfw_state {
                    a href=(show_hide_nsfw_uri) data-method="delete" {
                        i.fa.fa-eye-slash {}
                        "Hide NSFW streams"
                    }
                } @else {
                    a href=(show_hide_nsfw_uri) data-method="create" {
                        i.fa.fa-eye {}
                        "Show NSFW stream"
                    }
                }
            }

            .block__content {
                @for channel in channels.iter().filter(|x| x.nsfw == show_nsfw_state || !x.nsfw) {
                    (channel_box(&state, &mut client, &channel).await?)
                }
            }

            @if pages.need_pagination() {
                .block__header.page__header {
                    .page__pagination {
                        (pages.pagination())
                    }
                }
            }
        }
        br;
        //TODO: If can create channel
        h2 { "FAQ" }
        p {
            strong { "Q: Do you host streams?" }
            "A: No, we cheat and just link to streams on Picarto since that's where (almost) everyone is already. This is simply a nice way to track streaming artists."
        }
        p {
            strong { "Q: How do I get my stream/a friend's stream/<artist>'s stream here?" }
            "A: Send a private message to a site administrator with a link to the stream and the artist tag if applicable."
        }
    };
    let app = crate::pages::common::frontmatter::app(
        &state,
        &rstate,
        Some(PageTitle::from("Livestreams")),
        &mut client,
        body,
        None,
    )
    .await?;
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: app.into_string(),
    }))
}
