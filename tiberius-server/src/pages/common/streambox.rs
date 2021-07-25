use maud::html;
use maud::Markup;
use maud::PreEscaped;
use tiberius_core::error::TiberiusResult;
use tiberius_core::state::TiberiusRequestState;
use tiberius_models::Channel;
use tiberius_models::Client;
use rocket::Request;
use sqlx::{query_as, Acquire};
use std::str::FromStr;
use url::Url;

pub async fn stream_box(rstate: &TiberiusRequestState<'_>, client: &mut Client) -> TiberiusResult<Markup> {
    let channels: Vec<Channel> = Channel::get_frontpage_channels(client).await?;
    Ok(html! {
        @for channel in channels {
            div.block__content.flex.alternating-color {
                div.flex__grow {
                    a href=(channel_path(&channel)) {
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

pub fn channel_path(channel: &Channel) -> Url {
    let mut url = Url::from_str("https://picarto.tv/").unwrap();
    url.path_segments_mut().unwrap().push(&channel.short_name);
    url
}
