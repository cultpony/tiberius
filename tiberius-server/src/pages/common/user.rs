use axum_extra::routing::TypedPath;
use maud::{html, Markup, PreEscaped};
use tiberius_common_html::no_avatar_svg;
use tiberius_core::session::SessionMode;
use tiberius_core::state::TiberiusRequestState;
use tiberius_core::{error::TiberiusResult, session::Unauthenticated, state::TiberiusState};
use tiberius_models::{Client, Identifiable, IdentifiesUser, User};

use crate::pages::user::{PathUserAvatar, PathUserProfileId};

pub fn user_attribution_avatar(
    state: &TiberiusState,
    client: &mut Client,
    user: &Option<User>,
) -> Markup {
    html! {
        div.image-constrained."avatar--100px" {
            @match user {
                Some(user) => {
                    @match &user.avatar {
                        Some(avatar) => {
                            @let av_root = state.config.static_host::<Unauthenticated>(None);
                                img src=(format!("{}/avatars/{}", av_root, avatar.clone()));
                        },
                        None => (no_avatar_svg()),
                    }
                },
                None => (no_avatar_svg()),
            }
        }
    }
}

pub fn user_attribution_title(client: &mut Client, user: &Option<User>) -> Markup {
    //TODO:
    html! {}
}

/// Source_id must be set to the ID of the originating object
pub async fn user_attribution_main<I>(
    client: &mut Client,
    user: &Option<User>,
    source: I,
) -> TiberiusResult<Markup>
where
    I: Identifiable + IdentifiesUser,
{
    //TODO:
    Ok(html! {
        @match user {
            Some(user) => {
                @if !source.is_anonymous() {
                    strong {
                        @match source.user_id() {
                            Some(user_id) => {
                                a href=(PathUserProfileId{ user_id }) {
                                    @let user = User::get_id(client, user_id).await?.expect("user linked to comment does not exist");
                                    (user.displayname())
                                    // todo render awards
                                }
                            },
                            None => "Could not find user",
                        }
                    }
                } @else {
                    strong {
                        (anonymous_name(source.id(), source.best_user_identifier(client).await?, false))
                    }
                }
            },
            None => {
                strong {
                    (anonymous_name(source.id(), source.best_user_identifier(client).await?, false))
                }
            }
        }
    })
}

pub fn anonymous_name(identifier: i64, best_user_identifier: String, reveal_anon: bool) -> String {
    use tiberius_dependencies::{
        blake2::{Blake2s256, Digest},
        hex,
    };
    let mut hasher = Blake2s256::new();
    hasher.update(format!("id:{identifier};user:{best_user_identifier}"));
    let res = hasher.finalize();
    let res = hex::encode(res);
    let id = res.split_at(8).1;
    if reveal_anon {
        format!("{best_user_identifier} ({id}, hidden)")
    } else {
        format!("Background Pony {id}")
    }
}
