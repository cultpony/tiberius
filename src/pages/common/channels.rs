use crate::{
    app::HTTPReq,
    pages::common::{
        pluralize,
        routes::{artist_route, channel_route, path2url},
    },
};
use anyhow::Result;
use maud::{html, Markup};
use philomena_models::{Channel, Client};

pub async fn channel_box(req: &HTTPReq, client: &mut Client, channel: &Channel) -> Result<Markup> {
    let channel_route = path2url(req, channel_route(channel))?;
    let link_class = "media-box__header media-box__header--channel media-box__header--link";
    let artist_tag = channel.associated_artist_tag(client).await?;
    Ok(html! {
        .media-box {
            a.media-box__header.media-box__header--channel.media-box__header-link href=(channel_route) title=(channel.title()) {
                (channel.title())
            }
            .media-box__header.media-box__header--channel {
                @if channel.is_live {
                    .spacing-right.label.label--success.label--block.label--small {
                        strong { "LIVE NOW" }
                        (pluralize("viewer", "viewers", channel.viewers))
                    }
                } @else {
                    .label.label--danger.label--block.label--small {
                        strong { "OFF AIR" }
                    }
                }
            }

            @if channel.nsfw {
                .media-box__overlay { "NSFW" }
            }

            .media-box__content.media-box__content--channel {
                a href=(channel_route) {
                    .image-constrainted.media-box__content--channel {
                        img src=(channel.image()) alt=(channel.title());
                    }
                }
            }

            @if let Some(artist_tag) = artist_tag {
                a.(link_class) href=(path2url(req, artist_route(&artist_tag))?) {
                    i.fa.fa-fw.fa-tags { }
                    (artist_tag.name);
                }
            } @else {
                .media-box__header.media-box__header--channel { "No artist tag" }
            }
        }
    })
}
