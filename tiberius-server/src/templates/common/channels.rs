use crate::templates::common::{camoed_url, pluralize};
use crate::templates::tags::{PathTagsByNameShowTag, PathTagsShowTag};
use axum_extra::routing::TypedPath;
use maud::{html, Markup};
use tiberius_core::error::TiberiusResult;
use tiberius_core::state::TiberiusState;
use tiberius_models::{Channel, Client};

pub async fn channel_box(
    state: &TiberiusState,
    client: &mut Client,
    channel: &Channel,
) -> TiberiusResult<Markup> {
    let channel_route = "";
    let link_class = "media-box__header media-box__header--channel media-box__header--link";
    let artist_tag = channel.associated_artist_tag(client).await?;
    let header = html! {
        .media-box__header.media-box__header--channel {
            @if channel.is_live {
                .spacing-right.label.label--success.label--block.label--small {
                    strong { "LIVE NOW" }
                }
                (pluralize("viewer", "viewers", channel.viewers))
            } @else {
                .label.label--danger.label--block.label--small {
                    strong { "OFF AIR" }
                }
            }
        }
    };
    let title = html! {
        a.media-box__header.media-box__header--channel.media-box__header--link href=(channel_route) title=(channel.title()) {
            (channel.title())
        }
    };
    let content = html! {
        .media-box__content.media-box__content--channel {
            a href=(channel_route) {
                .image-constrained.media-box__content--channel {
                    img src=(camoed_url(state, &url::Url::parse(channel.image())?).await) alt=(channel.title());
                }
            }
        }
    };
    let artist = html! {
        @if let Some(artist_tag) = artist_tag {
            a.(link_class) href=(PathTagsShowTag{ tag_id: either::Either::Left(artist_tag.id as i64) }.to_uri().to_string()) {
                i.fa.fa-fw.fa-tags { }
                (artist_tag.name);
            }
        } @else {
            .media-box__header.media-box__header--channel { "No artist tag" }
        }
    };
    Ok(html! {
        .media-box {
            (title)

            (header)

            @if channel.nsfw {
                .media-box__overlay { "NSFW" }
            }

            (content)

            (artist)
        }
    })
}
