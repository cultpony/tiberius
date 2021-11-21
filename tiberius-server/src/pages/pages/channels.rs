use std::fmt;

use maud::{html, Markup};
use rocket::uri;
use rocket::{
    form::Form,
    http::{
        uri::{
            fmt::FromUriParam,
            fmt::{Formatter, Query, UriDisplay},
        },
        Cookie, CookieJar, Status,
    },
    response::Redirect,
    State,
};
use tiberius_core::app::PageTitle;
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::{
    ApiFormDataEmpty, FormMethod, HtmlResponse, RedirectResponse, TiberiusResponse,
};
use tiberius_core::session::SessionMode;
use tiberius_core::state::{Flash, TiberiusRequestState, TiberiusState};
use tiberius_models::{Channel, Client, Image};

use crate::pages::common::channels::channel_box;
use crate::pages::common::pagination::PaginationCtl;

#[derive(serde::Deserialize, rocket::FromForm)]
pub struct ChannelQuery {
    cq: Option<String>,
}

impl ChannelQuery {
    pub fn cq(&self) -> String {
        self.cq.clone().unwrap_or("".to_string())
    }
}

impl UriDisplay<Query> for ChannelQuery {
    fn fmt(&self, f: &mut Formatter<Query>) -> fmt::Result {
        f.write_named_value("cq", &self.cq)
    }
}

impl<'a, 'b> FromUriParam<Query, Option<&'a str>> for ChannelQuery {
    type Target = ChannelQuery;

    fn from_uri_param(cq: Option<&'a str>) -> ChannelQuery {
        ChannelQuery {
            cq: cq.map(|x| x.to_string()),
        }
    }
}

#[post("/channels/nsfw", data = "<fd>")]
pub async fn set_nsfw(
    rstate: TiberiusRequestState<'_, { SessionMode::Unauthenticated }>,
    fd: Form<ApiFormDataEmpty>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let fd = fd.into_afd();
    match fd.method() {
        Some(FormMethod::Create) => {
            rstate.cookie_jar.add(Cookie::new("chan_nsfw", "true"));
            Ok(TiberiusResponse::Redirect(RedirectResponse {
                redirect: Flash::alert("NSFW Channels are now visible")
                    .into_resp(Redirect::to(uri!(list_channels(cq = None)))),
            }))
        }
        Some(FormMethod::Delete) => {
            rstate.cookie_jar.add(Cookie::new("chan_nsfw", "false"));
            Ok(TiberiusResponse::Redirect(RedirectResponse {
                redirect: Flash::alert("NSFW Channels are now no longer visible")
                    .into_resp(Redirect::to(uri!(list_channels(cq = None)))),
            }))
        }
        _ => Ok(TiberiusResponse::Redirect(RedirectResponse {
            redirect: Flash::alert("Bad Request, is your browser up-to-date?")
                .into_resp(Redirect::to(uri!(list_channels(cq = None)))),
        })),
    }
}

#[post("/channels/read")]
pub async fn read() -> rocket::response::Redirect {
    todo!()
}

#[get("/channels?<cq>")]
pub async fn list_channels(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, { SessionMode::Unauthenticated }>,
    cq: Option<ChannelQuery>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let cq = cq.unwrap_or(ChannelQuery { cq: None });
    let mut client = state.get_db_client().await?;
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
    let show_hide_nsfw_uri = uri!(set_nsfw);
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
        state,
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
