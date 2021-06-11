use crate::{
    app::{HTTPReq, PageTitle},
    pages::common::{
        channels::channel_box,
        maud2tide,
        pagination::PaginationCtl,
        pluralize,
        routes::{artist_route, channel_nsfw_path, channel_route, path2url, todo_path},
        APIMethod,
    },
    request_helper::{ApiFormData, SafeSqlxRequestExt},
};
use anyhow::Result;
use maud::{html, Markup};
use philomena_models::{Channel, Client, Image};

#[derive(serde::Deserialize)]
pub struct ChannelQuery {
    cq: Option<String>,
}

impl ChannelQuery {
    pub fn cq(&self) -> String {
        self.cq.clone().unwrap_or("".to_string())
    }
}

#[post("/channels/nsfw")]
pub async fn set_nsfw(fd: ApiFormData<()>) -> rocket::response::Redirect {
    match fd.method() {
        _ => todo!(),
    }
}

#[post("/channels/read")]
pub async fn read() -> rocket::response::Redirect {
    todo!()
}

#[get("/channels")]
pub async fn list_channels() -> Markup {
    req.set_ext(PageTitle::from("Livestreams"));
    let mut client = req.get_db_client().await?;
    let channels = Channel::get_all_channels::<String>(&mut client, None).await?;
    //TODO: honor NSFW channel setting
    let pages = PaginationCtl::new(
        &req,
        &["cq"],
        Channel::count(&mut client, false).await?,
        "channels",
        "channel",
        "",
    )?;
    let cq: ChannelQuery = req.query()?;
    let body = html! {
        h1 { "Livestreams" }
        form.hform {
            .field {
                input.input.hform__text#channels_cq type="text" name="cq" value=(cq.cq()) placeholder="Search channels" autocapitalize="none";
                button.hform__button.button type="submit" { "Search" }
            }
        }
        .block {
            .block__header.page__header {
                .page__pagination {
                    (pages.pagination())
                }

                @if let Some(cookie) = req.cookie("chan_nsfw") {
                    @if cookie.value() == "true" {
                        a href=(path2url(&req, channel_nsfw_path(APIMethod::Delete))?) data-method="delete" {
                            i.fa.fa-eye-slash {}
                            "Hide NSFW streams"
                        }
                    }
                } else {
                    a href=(path2url(&req, channel_nsfw_path(APIMethod::Delete))?) data-method="create" {
                        i.fa.fa-eye {}
                        "Show NSFW stream"
                    }
                }
            }

            .block__content {
                @for channel in channels {
                    (channel_box(&req, &mut client, &channel).await?)
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
    let app = crate::pages::common::frontmatter::app(&mut req, client, body).await?;
    maud2tide(html! { (app) }, StatusCode::Ok)
}
