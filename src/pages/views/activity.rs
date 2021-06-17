use crate::{app::HTTPReq, config::Configuration, error::TiberiusResult, pages::common::{
        image::{image_block, image_block_default_sort, image_box, HeaderSize, ImageSize},
        streambox::stream_box,
    }, request_helper::SafeSqlxRequestExt};
use maud::{html, Markup};
use philomena_models::{Client, Image};
use rocket::{Request, State};

#[get("/")]
pub async fn html(request: Request<'_>, config: &State<Configuration>, client: &State<Client>) -> TiberiusResult<Markup> {
    let show_sidebar = true; //TODO: check setting
    let featured_image = Image::get_featured(&mut client).await?;
    let body = html! {
        div.column-layout {
            @if show_sidebar {
                aside.column-layout__left#activity-side {
                    @if let Some(featured_image) = featured_image {
                        @if !featured_image.hidden(&mut client)? {
                            .center {
                                h4.remove-top-margin { "Featured Image" }
                                (image_box(&request, &mut client, featured_image, ImageSize::Medium, HeaderSize::None).await?)
                            }
                        }
                    }
                    .block.block--fixed.block--fixed--sub.block--success.center.hide-mobile {
                        "Enjoy the site?"
                        a href="/pages/donations" { "Donate to keep it going!" }
                    }
                    .block.block--fixed.block--fixed--sub.center.hide-mobile {
                        "Issues? Want to chat?"
                        a href="/pages/contact" { "Contact us!" }
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href="/search?q=first_seen_at.gt:3 days ago&amp;sf=wilson_score&amp;sd=desc" {
                            "Trending Images"
                        }
                        .block__content.flex.flex--centered.flex--wrap.image-flex-grid {
                            @for image in Image::search(&mut client, "", Some("wilson_score"), Some("desc"), 0, 4).await?.1 {
                                (image_box(&request, &mut client, image, ImageSize::Medium, HeaderSize::ThumbSmall).await?)
                            }
                        }
                        a.block__header--single-item.center href="/search?q=*&amp;sf=score&amp;sd=desc" { "All Time Top Scoring" }
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href="/channels" { "Streams" }
                        (stream_box(&request, &mut client).await?)
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href="/forums" { "Forum Activity" }
                        //TODO: implement forum activity box
                    }
                    .block.hide-mobile {
                        a.block__header--single-item.center href="/comments" { "Recent Comments" }
                        //TODO: show recent comments
                        a.block__header--single-item.center href="/search?q=first_seen_at.gt:3 days ago&amp;sf=comment_count&amp;sd=desc" {
                            "Most Commented-on Images"
                        }
                    }
                }
                .column-layout__main {
                    (image_block_default_sort(&request, &mut client, "created_at.lte:3 minutes ago, processed:true", 0, 25, "recently uploaded").await?)
                }
            }
        }
    };
    Ok(html! {
        (crate::pages::common::frontmatter::app(&mut request, client, body).await?);
    })
}
