use maud::{html, PreEscaped};
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::SessionMode;
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tiberius_models::{Filter, User};

pub async fn filter_listing_item(
    filter: &Filter,
    state: &TiberiusState,
    current_user: Option<&User>,
) -> TiberiusResult<PreEscaped<String>> {
    let mut client = state.get_db_client();
    let user = filter.get_user(&mut client).await?;
    let user_filter = current_user
        .as_ref()
        .and_then(|x| x.user_settings.current_filter_id);
    Ok(html! {
        .filter {
            h3 { (filter.name()) }

            @if let Some(user) = user && !filter.system{
                p {
                    p {
                        "Maintained by " (user.displayname())
                    }
                }
            }

            @if filter.system {
                p {
                    "Maintained by staff"
                }
            }

            .filter-options {
                ul {
                    li {
                        "Spoilers "
                        (filter.spoilered_tag_ids.len())
                        ", hides "
                        (filter.hidden_tag_ids.len())
                    }

                    li {
                        a.button href="//TODO: link view filter" { "View this filter" }
                    }

                    li {
                        a.button href="//TODO: copy and customize" { "Copy and Customize" }
                    }

                    li {
                        a.button href="//TODO: edit filter" { "Edit this Filter" }
                    }

                    @if Some(filter.id) == user_filter {
                        li {
                            strong { "Your current filter" }
                        }
                    } @else {
                        a.button href="//TODO: change filter" { "Use this filter" }
                    }
                }
                p {
                    em {
                        (filter.description)
                    }
                }
            }
        }
    })
}
