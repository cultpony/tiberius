use std::fmt;

use axum::{extract::Query, response::Redirect, Extension, Form};
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

use crate::pages::common::{channels::channel_box, pagination::PaginationCtl};

#[derive(serde::Deserialize)]
pub struct ChannelQuery {
    cq: Option<String>,
}

impl ChannelQuery {
    pub fn cq(&self) -> String {
        self.cq.clone().unwrap_or("".to_string())
    }
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/channels/nsfw")]
pub struct PathSetChannelNsfw {}
pub async fn set_nsfw(
    mut rstate: TiberiusRequestState<Unauthenticated>,
    _: PathSetChannelNsfw,
    fd: Form<ApiFormDataEmpty>,
) -> TiberiusResult<(TiberiusRequestState<Unauthenticated>, Redirect)> {
    let fd = fd.into_afd();
    match fd.method() {
        Some(FormMethod::Create) => {
            rstate.cookie_jar = rstate.cookie_jar.add(Cookie::new("chan_nsfw", "true"));
            rstate.flash_mut().error("NSFW Channels are now visible");
            Ok((
                rstate,
                Redirect::to(PathChannelsList {}.to_uri().to_string().as_str()),
            ))
        }
        Some(FormMethod::Delete) => {
            rstate.cookie_jar = rstate.cookie_jar.add(Cookie::new("chan_nsfw", "false"));
            rstate
                .flash_mut()
                .error("NSFW Channels are now no longer visible");
            Ok((
                rstate,
                Redirect::to(PathChannelsList {}.to_uri().to_string().as_str()),
            ))
        }
        _ => Ok((
            rstate,
            Redirect::to(PathChannelsList {}.to_uri().to_string().as_str()),
        )),
    }
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/channels/read")]
pub struct PathChannelsRead {}

/// POST
pub async fn read(_: PathChannelsRead) -> String {
    todo!()
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/channels")]
pub struct PathChannelsList {}

pub async fn list_channels(
    Extension(state): Extension<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
    _: PathChannelsList,
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
    let show_hide_nsfw_uri = PathSetChannelNsfw {}.to_uri();
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
                .page__pagination {
                    (pages.pagination())
                }

                @if rstate.cookie_jar.get("chan_nsfw").map(|x| x.value()).unwrap_or("false") == "true" {
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
                @for channel in channels {
                    (channel_box(&mut client, &channel).await?)
                }
            }

            .block__header.page__header {
                .page__pagination {
                    (pages.pagination())
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
