use std::marker::PhantomData;

use axum::extract::{FromRequest, RequestParts};
use reqwest::StatusCode;
use tiberius_dependencies::reqwest;
use tiberius_dependencies::{casbin, casbin::prelude::*};

use crate::{
    error::TiberiusResult,
    session::{Authenticated, SessionMode, Unauthenticated},
    state::{TiberiusRequestState, TiberiusState},
};

pub struct ACLEntity<A, B, C>(A, B, C)
where
    A: ACLSubjectTrait,
    B: ACLObjectTrait,
    C: ACLActionTrait;

impl<A: ACLSubjectTrait, B: ACLObjectTrait, C: ACLActionTrait> From<(A, B, C)>
    for ACLEntity<A, B, C>
{
    fn from((a, b, c): (A, B, C)) -> Self {
        ACLEntity(a, b, c)
    }
}

#[derive(Clone, PartialEq)]
pub enum ACLSubject {
    /// No user given
    None,
    /// An anonymous User (the permission is only given based on login, not on the actual user)
    Anonymous,
    /// A logged in and active user
    User(tiberius_models::User),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLObject {
    /// Global Permissions
    Site,
    /// An image that has been uploaded or is processing
    Image,
    /// A filter used to hide or change which images are visible/spoilered
    Filter,
    /// A permanent sessionkey
    APIKey,
    /// Staff Category for the Staff Page
    StaffCategory,
    /// Staff Entry into the Staff Page
    StaffUserEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLActionFilter {
    /// Edit filters the user owns
    EditOwned,
    /// Edit filters the user does not own
    EditAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLActionSite {
    /// Allow to view the site, if not given require login
    Use,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLActionImage {
    ChangeUploader,
    MergeDuplicate,
    IncrementView,
    RepairImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLActionAPIKey {
    ViewAll,
    CreateDelete,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLActionStaffCategory {
    Manage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLActionStaffUserEntry {
    EditSelf,
    Admin,
}

pub trait ACLSubjectTrait {
    fn subject(&self) -> String;
}

pub trait ACLObjectTrait: std::fmt::Debug {
    fn object(&self) -> String;
    fn inner(&self) -> &ACLObject;
}

pub trait ACLActionTrait: std::fmt::Debug {
    fn action(&self) -> String;
    fn action_of(&self, _: &ACLObject) -> bool;
}

impl ACLObjectTrait for ACLObject {
    fn object(&self) -> String {
        match self {
            ACLObject::Site => "site",
            ACLObject::Image => "image",
            ACLObject::APIKey => "api_key",
            ACLObject::StaffCategory => "staff_category",
            ACLObject::StaffUserEntry => "staff_user_entry",
            ACLObject::Filter => "filter",
        }
        .to_string()
    }
    fn inner(&self) -> &ACLObject {
        self
    }
}

impl ACLSubjectTrait for ACLSubject {
    fn subject(&self) -> String {
        match self {
            ACLSubject::User(v) => {
                format!("user::{}", v.email)
            }
            ACLSubject::Anonymous => "any::user".to_string(),
            ACLSubject::None => "anonymous::anonymous".to_string(),
        }
    }
}

impl ACLActionTrait for ACLActionSite {
    fn action(&self) -> String {
        match self {
            ACLActionSite::Use => "use".to_string(),
        }
    }

    fn action_of(&self, a: &ACLObject) -> bool {
        *a == ACLObject::Site
    }
}

impl ACLActionTrait for ACLActionImage {
    fn action(&self) -> String {
        match self {
            ACLActionImage::ChangeUploader => "change_uploader".to_string(),
            ACLActionImage::MergeDuplicate => "merge_duplicate".to_string(),
            ACLActionImage::IncrementView => "increment_view".to_string(),
            ACLActionImage::RepairImage => "repair_image".to_string(),
        }
    }
    fn action_of(&self, a: &ACLObject) -> bool {
        *a == ACLObject::Image
    }
}

impl ACLActionTrait for ACLActionAPIKey {
    fn action(&self) -> String {
        match self {
            ACLActionAPIKey::ViewAll => "view".to_string(),
            ACLActionAPIKey::CreateDelete => "create_delete".to_string(),
            ACLActionAPIKey::Admin => "admin".to_string(),
        }
    }
    fn action_of(&self, a: &ACLObject) -> bool {
        *a == ACLObject::APIKey
    }
}

impl ACLActionTrait for ACLActionStaffCategory {
    fn action(&self) -> String {
        match self {
            ACLActionStaffCategory::Manage => "manage",
        }
        .to_string()
    }
    fn action_of(&self, a: &ACLObject) -> bool {
        *a == ACLObject::StaffCategory
    }
}

impl ACLActionTrait for ACLActionStaffUserEntry {
    fn action(&self) -> String {
        match self {
            ACLActionStaffUserEntry::Admin => "admin",
            ACLActionStaffUserEntry::EditSelf => "edit_self",
        }
        .to_string()
    }
    fn action_of(&self, a: &ACLObject) -> bool {
        *a == ACLObject::StaffUserEntry
    }
}

impl ACLActionTrait for ACLActionFilter {
    fn action(&self) -> String {
        match self {
            ACLActionFilter::EditOwned => "edit_own",
            ACLActionFilter::EditAll => "admin",
        }.to_string()
    }

    fn action_of(&self, a: &ACLObject) -> bool {
        *a == ACLObject::Filter
    }
}

#[instrument(skip(state, rstate), fields(user = rstate.session().raw_user()))]
pub async fn verify_acl<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
    object: impl ACLObjectTrait,
    action: impl ACLActionTrait,
) -> TiberiusResult<bool> {
    assert!(
        action.action_of(object.inner()),
        "ACL Action {:?} was not member of ACL Object {:?}",
        object,
        action
    );
    let subject = rstate.user(state).await?;
    let subject = match subject {
        None => ACLSubject::None,
        Some(v) => ACLSubject::User(v.clone()),
    };
    let v = (subject.subject(), object.object(), action.action());
    debug!("Checking if {:?} is OK in RBAC", v);
    let casbin: casbin::Enforcer = state.get_acl_enforcer().await?;
    let enforce_result = casbin.enforce(v.clone())?;
    info!("Result of {:?} = {:?}", v, enforce_result);
    Ok(enforce_result)
}
