use maud::{html, Markup};
use tiberius_core::{
    error::{TiberiusError, TiberiusResult},
    state::TiberiusState,
};
use tiberius_models::{comment::Comment, Client, Identifiable, Image, User};

use crate::templates::common::{
    frontmatter::pretty_time,
    renderer::textile::render_textile,
    user::{user_attribution_avatar, user_attribution_main, user_attribution_title},
};

/// Renders a list of the comments under the given image
pub async fn comment_view(
    state: &TiberiusState,
    client: &mut Client,
    image: &Image,
) -> TiberiusResult<Markup> {
    let comments = image.comments(client).await?;
    Ok(html! {
        @for comment in comments {
            (single_comment(state, client, &comment).await?)
        }
    })
}

/// Renders the Comment Creation Form or the ban message if the user is banned
pub async fn comment_form(
    client: &mut Client,
    user: &Option<User>,
    image: &Image,
) -> TiberiusResult<Markup> {
    // TODO: check if banned
    // TODO: check if commenting allowed
    // TODO: render forms
    Ok(html! {})
}

pub async fn single_comment(
    state: &TiberiusState,
    client: &mut Client,
    comment: &Comment,
) -> TiberiusResult<Markup> {
    let author = comment.author(client).await?;
    Ok(state
        .comment_cache
        .try_get_with(comment.id() as u64, async {
            Ok::<Markup, TiberiusError>(html! {
                article.block.communication id=(format!("comment_{}", comment.id)) {
                    div.block__content.flex."flex--no-wrap".(communication_body_class(comment)) {
                        .flex__fixed.spacing-right {
                            (user_attribution_avatar(state, client, &author))
                        }
                        .flex__grow.communication_body {
                            span.communication__body__sender-name {
                                (user_attribution_main(client, &author, comment).await?)
                            }
                            br;
                            (user_attribution_title(client, &author));
                            .communication__body__text {
                                @if comment.hidden_from_users {
                                    strong.comment_deleted {
                                        "Deletion reason: "
                                        (comment.deletion_reason)
                                        // TODO: add undelete and mod controls
                                    }
                                } else {
                                    // TODO: render markdown
                                    (render_textile(&comment.body))
                                }
                            }
                        }
                    }
                    div.block__content.communication__options {
                        .flex.flex--wrap.flex--spaced-out {
                            (comment_options(comment))
                        }
                        // TODO: staff actions
                    }
                }
            })
        })
        .await
        .expect("could not render comment"))
}

pub fn communication_body_class(comment: &Comment) -> &'static str {
    match comment.destroyed_content {
        Some(true) => "communication--destroyed",
        _ => "",
    }
}

pub fn comment_options(comment: &Comment) -> Markup {
    html! {
        div {
            "Posted "
            (pretty_time(&comment.created_at))
        }
    }
}
