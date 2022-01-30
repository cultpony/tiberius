use casbin::CoreApi;
use rocket::State;
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::{Authenticated, Unauthenticated, SessionMode};
use tiberius_core::state::{TiberiusState, TiberiusRequestState};


#[derive(Clone, PartialEq)]
pub enum ACLSubject {
    None,
    User(tiberius_models::User),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLObject {
    Image,
    APIKey,
    StaffCategory,
    StaffUserEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ACLActionImage {
    ChangeUploader,
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
            ACLObject::Image => "image",
            ACLObject::APIKey => "api_key",
            ACLObject::StaffCategory => "staff_category",
            ACLObject::StaffUserEntry => "staff_user_entry",
        }.to_string()
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
            ACLSubject::None => "anonymous::anonymous".to_string(),
        }
    }
}

impl ACLActionTrait for ACLActionImage {
    fn action(&self) -> String {
        match self {
            ACLActionImage::ChangeUploader => "change_uploader".to_string(),
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
            ACLActionStaffCategory::Manage => "manage"
        }.to_string()
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
        }.to_string()
    }
    fn action_of(&self, a: &ACLObject) -> bool {
        *a == ACLObject::StaffUserEntry
    }
}

pub async fn verify_acl<T: SessionMode>(
    state: &State<TiberiusState>,
    rstate: &TiberiusRequestState<'_, T>,
    object: impl ACLObjectTrait,
    action: impl ACLActionTrait,
) -> TiberiusResult<bool> {
    assert!(action.action_of(object.inner()), "ACL Action {:?} was not member of ACL Object {:?}", object, action);
    let casbin = state.get_casbin();
    let subject = rstate.user(state).await?;
    let subject = match subject {
        None => ACLSubject::None,
        Some(v) => ACLSubject::User(v),
    };
    let v = (subject.subject(), object.object(), action.action());
    debug!("Checking if {:?} is OK in RBAC", v);
    let enforce_result = casbin.read().await.enforce(v.clone())?;
    debug!("Result of {:?} = {:?}", v, enforce_result);
    Ok(enforce_result)
}