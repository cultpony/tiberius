use axum::{response::Redirect, Extension, Form, extract::State};
use axum_extra::routing::TypedPath;
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use tiberius_core::{
    acl::*,
    error::TiberiusResult,
    request_helper::{HtmlResponse, RedirectResponse, TiberiusResponse},
    session::{Authenticated, Unauthenticated},
    state::{PageSubtextCacheTag, TiberiusRequestState, TiberiusState},
};
use tiberius_dependencies::axum_flash::Flash;
use tiberius_models::{Client, StaffCategory, StaffCategoryColor, User, UserStaffEntry};

use crate::pages::common::frontmatter::{
    csrf_input_tag, form_submit_button, user_attribution, user_attribution_avatar,
};

#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/pages/staff")]
pub struct PathShowStaffPage {}

#[instrument(skip(state, rstate))]
pub async fn show(
    State(state): State<TiberiusState>,
    rstate: TiberiusRequestState<Unauthenticated>,
    _: PathShowStaffPage,
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client: Client = state.get_db_client();
    let categories = StaffCategory::get_all(&mut client).await?;
    trace!("Got categories: {:?}", categories);
    let entries = UserStaffEntry::get_all(&mut client).await?;
    let cat_ent: Vec<(StaffCategory, Vec<&UserStaffEntry>)> = categories
        .into_iter()
        .map(|category| {
            let id = category.id;
            (
                category,
                entries
                    .iter()
                    .filter(|entry| entry.staff_category_id == id)
                    .collect(),
            )
        })
        .collect();
    let current_user = rstate.user(&state).await?;
    let acl_can_admin_categories = verify_acl(
        &state,
        &rstate,
        ACLObject::StaffCategory,
        ACLActionStaffCategory::Manage,
    )
    .await?;
    let acl_can_edit_all_user = verify_acl(
        &state,
        &rstate,
        ACLObject::StaffUserEntry,
        ACLActionStaffUserEntry::EditSelf,
    )
    .await?;
    let acl_can_edit_own_user = verify_acl(
        &state,
        &rstate,
        ACLObject::StaffUserEntry,
        ACLActionStaffUserEntry::Admin,
    )
    .await?;
    let can_edit_this = |user: &UserStaffEntry| -> bool {
        (acl_can_edit_own_user && Some(user.user_id) == current_user.as_ref().map(|x| x.id as i64))
            || acl_can_edit_all_user
    };
    let cache_key = {
        match current_user.as_ref() {
            Some(v) => (true, v.id),
            None => (false, 0),
        }
    };
    #[cfg(feature = "everyone-can-pm")]
    let pm_button = html! { .staff-block__info {
        a.button href="" {
            i.fa.fa-envelope {}
            "Send PM"
        }
    } };
    #[cfg(not(feature = "everyone-can-pm"))]
    let pm_button = html! {};
    let body = state.page_subtext_cache.try_get_with(PageSubtextCacheTag::StaffPageContent{logged_in: cache_key.0, user: cache_key.1}, async {
        let v: TiberiusResult<PreEscaped<String>> = Ok(html! {
            h1 { "Staff" }
            div.block.block--fixed.block--warning {
                h3 { "Do you wish to submit a report?" }
                p {
                    strong {
                        "Do "
                        em { "not" }
                        " contact staff members with your reports. Instead, if you think something breaks "
                        a href="/pages/rules" { "the rules"}
                        " use the \"Report\" button, which is included next to all user-created content on the site."
                        "This will ensure swift handling of your issue, since most staff members don't check their PMs nearly as vigilantly as the reports queue."
                    }
                }
                /*p {
                    "Staff PMs are only for general questions or for getting help with using the site."
                }*/
            }
            div.block.block--fixed {
                p {
                    "Before contacting any of the staff members, you should try to ask your question in our "
                    a href="/pages/discord" { "Discord"}
                }
                p {
                    "Keep in mind that all staff are unpaid volunteers who donate their time and effort into making sure this site remains organized and operational."
                    " "
                    "Please do not harass them, and try to keep your PMs constructive."
                    " "
                    "We will happily answer your questions, however receiving plenty of PMs for no reason gets tiring and impacts our ability to tend to more important matters,"
                    " "
                    "so please make sure you actually have a need to contact a staff member before doing so."
                }
            }
            @if acl_can_admin_categories {
                div.block.block--fixed {
                    form action=(PathNewCategory{}.to_uri().to_string()) method="POST" {
                        (csrf_input_tag(&rstate).await);
                        .field {
                            input.input #category_name name="name" type="text" required="true" placeholder="Name";
                        }
                        .field {
                            input.input #category_role name="role" type="text" required="true" placeholder="Role required for Category";
                        }
                        .field {
                            input.input #category_ordering name="ordering" type="number" required="true" placeholder="Ordering";
                        }
                        .field {
                            select.input #category_color name="color" {
                                option value="green" { "Green" }
                                option value="green" { "Orange" }
                                option value="green" { "Red" }
                                option value="green" { "Purple" }
                                option value="green" selected="1" { "None" }
                            }
                        }
                        .field {
                            input.input #category_text name="text" type="text" required="true" placeholder="Description";
                        }
                        (form_submit_button("Create Category"));
                    }
                }
            }
            div.staff-block {
                @for (header, users) in cat_ent {
                    div class=(format!("block block--fixed staff-block__category {}", header.category().to_string())) { (header.display_name) }
                    p.staff-block__description {
                        i.fa.fa-fw.fa-info-circle {}
                        (header.text)
                    }
                    @if users.len() > 0 {
                        .staff-block__grid {
                            @for user in users {
                                .block.flex.flex--column {
                                    .block__content.staff-block__user {
                                        @if can_edit_this(user) {
                                            div.block.block--fixed {
                                                form action=(PathEditUserEntry{ entry_id: user.user_id }.to_uri().to_string()) method="POST" {
                                                        (csrf_input_tag(&rstate).await);
                                                        .field {
                                                            input.input #user_email name="display_name" type="text" placeholder="Display Name" value=(user.display_name.as_ref().unwrap_or(&"".to_string())) required="true";
                                                        }
                                                        .field {
                                                            input.input #user_email name="text" type="text" placeholder="Description" value=(user.text.as_ref().unwrap_or(&"".to_string())) required="true";
                                                        }
                                                        (form_submit_button(&format!("Edit {} in category {}", user.display_name(&mut client).await?, header.display_name)));
                                                }
                                            }
                                        }
                                        @if user.unavailable {
                                            .staff-block__user-card {
                                                a.profile-block href="" { (user_attribution_avatar(&user.get_user(&mut client).await?.unwrap(), "avatar--125px avatar-disabled")?) }
                                                p {
                                                    b.staff-title-muted {
                                                        (user.display_name(&mut client).await?)
                                                    }
                                                    p { " (Unavailable)" }
                                                }
                                            }
                                        } else {
                                            .staff-block__user-card {
                                                a.profile-block href="" { (user_attribution_avatar(&user.get_user(&mut client).await?.unwrap(), "avatar--125px")?) }
                                                p {
                                                    b {
                                                        (user.display_name(&mut client).await?)
                                                    }
                                                    p { (user.text.as_ref().unwrap_or(&"".to_string())) }
                                                }
                                            }
                                            (pm_button)
                                        }
                                    }
                                }
                            }
                        }
                    } else if acl_can_admin_categories {
                    }
                    @if acl_can_admin_categories {
                        form action=(PathAddUserToCategory{ category: header.id }.to_uri().to_string()) method="POST" {
                                (csrf_input_tag(&rstate).await);
                                .field {
                                    input.input #user_email name="user_email" type="email" placeholder="User E-Mail" required="true";
                                }
                                (form_submit_button(&format!("Add user to {}", header.display_name)));
                        }
                    }
                }
            }
        });
        v
    }).await;
    let body = match body {
        Ok(v) => v,
        Err(e) => panic!("{}", e), //TODO: don't panic on this
    };
    let page: PreEscaped<String> = html! {
        (crate::pages::common::frontmatter::app(&state, &rstate, None, &mut client, body, None).await?);
    };
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: page.into_string(),
    }))
}

#[derive(Deserialize, Debug)]
pub struct NewCategoryRequest {
    role: String,
    ordering: i64,
    color: StaffCategoryColor,
    name: String,
    text: String,
}

#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/pages/staff/category")]
pub struct PathNewCategory {}

#[tracing::instrument]
pub async fn new_category(
    _: PathNewCategory,
    State(state): State<TiberiusState>,
    flash: Flash,
    rstate: TiberiusRequestState<Authenticated>,
    new_category_request: Form<NewCategoryRequest>,
) -> TiberiusResult<(Flash, Redirect)> {
    let mut client: Client = state.get_db_client();
    let acl_can_admin_categories = verify_acl(
        &state,
        &rstate,
        ACLObject::StaffCategory,
        ACLActionStaffCategory::Manage,
    )
    .await?;
    if !acl_can_admin_categories {
        todo!("Deny API access");
    }
    let mut cat = StaffCategory {
        role: new_category_request.role.to_string(),
        ordering: new_category_request.ordering,
        color: new_category_request.color,
        display_name: new_category_request.name.to_string(),
        text: new_category_request.text.to_string(),
        ..Default::default()
    };

    cat.save(&mut client).await?;

    let current_user = rstate.user(&state).await?;
    state
        .page_subtext_cache
        .invalidate(&PageSubtextCacheTag::staff_page_content(&current_user))
        .await;

    Ok((flash.error(format!(
        "Created new category {} ({})",
        cat.display_name, cat.id
    )), Redirect::to(
        PathShowStaffPage {}.to_uri().to_string().as_str(),
    )))
}

#[derive(Deserialize, Debug)]
pub struct AddUserCategoryRequest {
    user_email: String,
}

#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/pages/staff/category/:category")]
pub struct PathAddUserToCategory {
    category: i64,
}

#[tracing::instrument]
pub async fn add_user_to_category(
    PathAddUserToCategory { category }: PathAddUserToCategory,
    State(state): State<TiberiusState>,
    flash: Flash,
    rstate: TiberiusRequestState<Authenticated>,
    new_user_request: Form<AddUserCategoryRequest>,
) -> TiberiusResult<(Flash, Redirect)> {
    let mut client: Client = state.get_db_client();
    let acl_can_admin_categories = verify_acl(
        &state,
        &rstate,
        ACLObject::StaffCategory,
        ACLActionStaffCategory::Manage,
    )
    .await?;
    if !acl_can_admin_categories {
        todo!("Deny API access");
    }

    let user =
        if let Some(user) = User::get_by_email(&mut client, &new_user_request.user_email).await? {
            user
        } else {
            todo!("User does not exist");
        };

    let mut cat_user = UserStaffEntry {
        user_id: user.id as i64,
        staff_category_id: category,
        display_name: Some(user.displayname().to_string()),
        text: Some("".to_string()),
        ..Default::default()
    };

    cat_user.save(&mut client).await?;

    let current_user = rstate.user(&state).await?;
    state
        .page_subtext_cache
        .invalidate(&PageSubtextCacheTag::staff_page_content(&current_user))
        .await;

    Ok((flash.error(format!(
        "Add user {:?} ({}) to category {}",
        cat_user.display_name, cat_user.id, cat_user.staff_category_id
    )), Redirect::to(
        PathShowStaffPage {}.to_uri().to_string().as_str(),
    )))
}

#[derive(Debug)]
pub struct EditUserCategoryRequest {
    display_name: String,
    text: String,
}

#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/pages/staff/entry/:entry_id")]
pub struct PathEditUserEntry {
    entry_id: i64,
}

#[tracing::instrument]
pub async fn edit_user_entry(
    State(state): State<TiberiusState>,
    flash: Flash,
    rstate: TiberiusRequestState<Authenticated>,
    PathEditUserEntry { entry_id }: PathEditUserEntry,
    edit_user_request: Form<EditUserCategoryRequest>,
) -> TiberiusResult<(Flash, Redirect)> {
    let mut client: Client = state.get_db_client();
    let acl_can_admin_categories = verify_acl(
        &state,
        &rstate,
        ACLObject::StaffCategory,
        ACLActionStaffCategory::Manage,
    )
    .await?;
    let acl_can_edit_all_user = verify_acl(
        &state,
        &rstate,
        ACLObject::StaffUserEntry,
        ACLActionStaffUserEntry::EditSelf,
    )
    .await?;
    let acl_can_edit_own_user = verify_acl(
        &state,
        &rstate,
        ACLObject::StaffUserEntry,
        ACLActionStaffUserEntry::Admin,
    )
    .await?;
    let current_user = rstate.user(&state).await?;
    let can_edit_this = |user: &UserStaffEntry| -> bool {
        (acl_can_edit_own_user && Some(user.user_id) == current_user.as_ref().map(|x| x.id as i64))
            || acl_can_edit_all_user
    };
    if !acl_can_admin_categories {
        todo!("Deny API access");
    }

    let mut entry = UserStaffEntry::get_by_id(entry_id, &mut client).await?;

    match &mut entry {
        None => {
            let entry = UserStaffEntry {
                display_name: Some(edit_user_request.display_name.to_string()),
                ..Default::default()
            };
            todo!("save entry")
        }
        Some(entry) => {
            entry.display_name = Some(edit_user_request.display_name.to_string());
            entry.text = Some(edit_user_request.text.to_string());

            entry.save(&mut client).await?;
        }
    }

    state
        .page_subtext_cache
        .invalidate(&PageSubtextCacheTag::staff_page_content(&current_user))
        .await;

    Ok((flash.error("Entry saved"), Redirect::to(
        PathShowStaffPage {}.to_uri().to_string().as_str(),
    )))
}

// TODO: allow user to hide from staff list
// TODO: allow user to link primary account instead of true staff account
