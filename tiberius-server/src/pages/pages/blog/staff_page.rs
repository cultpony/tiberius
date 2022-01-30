use maud::{Markup, html, PreEscaped};
use rocket::State;
use rocket::form::Form;
use rocket::response::Redirect;
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::{HtmlResponse, TiberiusResponse, RedirectResponse};
use tiberius_core::session::{Unauthenticated, Authenticated};
use tiberius_core::state::{TiberiusState, TiberiusRequestState, Flash};
use tiberius_models::{Client, StaffCategory, UserStaffEntry, StaffCategoryColor, User};

use crate::pages::common::acl::{verify_acl, ACLObject, ACLActionStaffCategory, ACLActionStaffUserEntry};
use crate::pages::common::frontmatter::{csrf_input_tag, form_submit_button, user_attribution, user_attribution_avatar};

#[get("/pages/staff")]
pub async fn show(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Unauthenticated>
) -> TiberiusResult<TiberiusResponse<()>> {
    let mut client: Client = state.get_db_client().await?;
    let categories = StaffCategory::get_all(&mut client).await?;
    trace!("Got categories: {:?}", categories);
    let entries = UserStaffEntry::get_all(&mut client).await?;
    let cat_ent: Vec<(StaffCategory, Vec<&UserStaffEntry>)> = categories.into_iter().map(|category| {
        let id = category.id;
        (category, entries.iter().filter(|entry| entry.staff_category_id == id).collect())
    }).collect();
    let current_user = rstate.user(&state).await?;
    let acl_can_admin_categories = verify_acl(
        state, &rstate,
        ACLObject::StaffCategory,
        ACLActionStaffCategory::Manage,
    ).await?;
    let acl_can_edit_all_user = verify_acl(
        state, &rstate,
        ACLObject::StaffUserEntry,
        ACLActionStaffUserEntry::EditSelf,
    ).await?;
    let acl_can_edit_own_user = verify_acl(
        state, &rstate,
        ACLObject::StaffUserEntry,
        ACLActionStaffUserEntry::Admin,
    ).await?;
    let can_edit_this = |user: &UserStaffEntry| -> bool {
        (acl_can_edit_own_user && Some(user.user_id) == current_user.as_ref().map(|x| x.id as i64))
        || acl_can_edit_all_user
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
    let body = html! {
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
                "Please do not harass them, and try to keep your PMs constructive. "
                "We will happily answer your questions, however receiving plenty of PMs for no reason gets tiring and impacts our ability to tend to more important matters, "
                "so please make sure you actually have a need to contact a staff member before doing so."
            }
        }
        @if acl_can_admin_categories {
            div.block.block--fixed {
                form action=(uri!(new_category)) method="POST" {
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
                                            form action=(uri!(edit_user_entry(user.user_id))) method="POST" {
                                                    (csrf_input_tag(&rstate).await);
                                                    .field {
                                                        input.input #user_email name="display_name" type="text" placeholder="Display Name" required="true";
                                                    }
                                                    .field {
                                                        input.input #user_email name="text" type="text" placeholder="Description" required="true";
                                                    }
                                                    (form_submit_button(&format!("Add user to {}", header.display_name)));
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
                                                " (Unavailable)"
                                            }
                                        }
                                    } else {
                                        .staff-block__user-card {
                                            a.profile-block href="" { (user_attribution_avatar(&user.get_user(&mut client).await?.unwrap(), "avatar--125px")?) }
                                            p {
                                                b {
                                                    (user.display_name(&mut client).await?)
                                                }
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
                    form action=(uri!(add_user_to_category(header.id))) method="POST" {
                            (csrf_input_tag(&rstate).await);
                            .field {
                                input.input #user_email name="user_email" type="email" placeholder="User E-Mail" required="true";
                            }
                            (form_submit_button(&format!("Add user to {}", header.display_name)));
                    }
                }
            }
        }
    };
    let page: PreEscaped<String> = html! {
        (crate::pages::common::frontmatter::app(state, &rstate, None, &mut client, body, None).await?);
    };
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: page.into_string(),
    }))
}

#[derive(FromForm)]
pub struct NewCategoryRequest<'r> {
    role: &'r str,
    ordering: i64,
    color: StaffCategoryColor,
    name: &'r str,
    text: &'r str,
}

#[post("/pages/staff/category", data = "<new_category_request>")]
pub async fn new_category(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Authenticated>,
    new_category_request: Form<NewCategoryRequest<'_>>,
) -> TiberiusResult<RedirectResponse> {
    let mut client: Client = state.get_db_client().await?;
    let acl_can_admin_categories = verify_acl(
        state, &rstate,
        ACLObject::StaffCategory,
        ACLActionStaffCategory::Manage,
    ).await?;
    if !acl_can_admin_categories {
        todo!("Deny API access");
    }
    let mut cat = StaffCategory{
        role: new_category_request.role.to_string(),
        ordering: new_category_request.ordering,
        color: new_category_request.color,
        display_name: new_category_request.name.to_string(),
        text: new_category_request.text.to_string(),
        ..Default::default()
    };

    cat.save(&mut client).await?;

    Ok(RedirectResponse {
        redirect: Flash::alert(format!("Created new category {} ({})", cat.display_name, cat.id))
            .into_resp(Redirect::to(uri!(show))),
    })
}

#[derive(FromForm)]
pub struct AddUserCategoryRequest<'r> {
    user_email: &'r str,
}

#[post("/pages/staff/category/<category>", data = "<new_user_request>")]
pub async fn add_user_to_category(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Authenticated>,
    category: i64,
    new_user_request: Form<AddUserCategoryRequest<'_>>,
) -> TiberiusResult<RedirectResponse> {
    let mut client: Client = state.get_db_client().await?;
    let acl_can_admin_categories = verify_acl(
        state, &rstate,
        ACLObject::StaffCategory,
        ACLActionStaffCategory::Manage,
    ).await?;
    if !acl_can_admin_categories {
        todo!("Deny API access");
    }

    let user = if let Some(user) = User::get_by_email(&mut client, new_user_request.user_email).await? {
        user
    } else {
        todo!("User does not exist");
    };

    let mut cat_user = UserStaffEntry{
        user_id: user.id as i64,
        staff_category_id: category,
        display_name: Some(user.displayname().to_string()),
        text: Some("".to_string()),
        ..Default::default()
    };

    cat_user.save(&mut client).await?;

    Ok(RedirectResponse {
        redirect: Flash::alert(format!("Add user {:?} ({}) to category {}", cat_user.display_name, cat_user.id, cat_user.staff_category_id))
            .into_resp(Redirect::to(uri!(show))),
    })
}

#[derive(FromForm)]
pub struct EditUserCategoryRequest<'r> {
    display_name: &'r str,
    text: &'r str,
}

#[post("/pages/staff/entry/<entry_id>", data = "<edit_user_request>")]
pub async fn edit_user_entry(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Authenticated>,
    entry_id: i64,
    edit_user_request: Form<EditUserCategoryRequest<'_>>,
) -> TiberiusResult<RedirectResponse> {
    let mut client: Client = state.get_db_client().await?;
    let acl_can_admin_categories = verify_acl(
        state, &rstate,
        ACLObject::StaffCategory,
        ACLActionStaffCategory::Manage,
    ).await?;
    let acl_can_edit_all_user = verify_acl(
        state, &rstate,
        ACLObject::StaffUserEntry,
        ACLActionStaffUserEntry::EditSelf,
    ).await?;
    let acl_can_edit_own_user = verify_acl(
        state, &rstate,
        ACLObject::StaffUserEntry,
        ACLActionStaffUserEntry::Admin,
    ).await?;
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
        None => todo!("missing entry"),
        Some(entry) => {
            entry.display_name = Some(edit_user_request.display_name.to_string());
            entry.text = Some(edit_user_request.text.to_string());

            entry.save(&mut client).await?;
        }
    }

    Ok(RedirectResponse {
        redirect: Flash::alert("Entry saved")
            .into_resp(Redirect::to(uri!(show))),
    })
}
// TODO: Add user to category
// TODO: allow user to hide from staff list
// TODO: allow user to link primary account instead of true staff account