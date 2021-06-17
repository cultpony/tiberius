use crate::app::HTTPReq;
use crate::error::TiberiusResult;
use maud::html;
use maud::Markup;
use maud::PreEscaped;
use philomena_models::Channel;
use philomena_models::Client;
use rocket::Request;
use sqlx::{query_as, Acquire};
use std::str::FromStr;
use url::Url;

pub async fn stream_box(req: &Request<'_>, client: &mut Client) -> TiberiusResult<Markup> {
    let channels: Vec<Channel> = {
        query_as!(
            Channel,
            r#"
                SELECT * FROM channels 
                WHERE 
                    nsfw = false
                AND
                    last_fetched_at is not null"#
        )
        .fetch_all(client.db().acquire().await?)
        .await?
    };
    Ok(html! {
        @for channel in channels {
            div.block__content.flex.alternating-color {
                div.flex__grow {
                    a href=(channel_path(&req, &channel)) {
                        @match channel.channel_image {
                            Some(channel_image_url) => {
                                img width="16" height="16"
                                    src=(channel_image_url)
                                    alt=(channel.short_name)
                                    referrerpolicy="no-referrer"
                                    style=(PreEscaped("margin-right: 0.5em"));
                            },
                            None => {
                                img width="16" height="16"
                                    src="/images/no_avatar.svg"
                                    alt=(channel.short_name)
                                    style=(PreEscaped("margin-right: 0.5em"));
                            },
                        }
                        @if channel.title.is_empty() {
                            (channel.short_name);
                        } @else {
                            (channel.title);
                        }
                    }
                }
                div.flex__fixed.flex__right {
                    @if channel.is_live {
                        span.channel-strip__state.label.label--narrow.label--success {
                            "LIVE NOW";
                        }
                    } @else {
                        span.channel-strip__state.label.label--narrow.label--danger {
                            "OFF AIR";
                        }
                    }
                }
            }
        }
    })
}

pub fn channel_path(req: &HTTPReq, channel: &Channel) -> Url {
    let mut url = Url::from_str("https://picarto.tv/").unwrap();
    url.path_segments_mut().unwrap().push(&channel.short_name);
    url
}
