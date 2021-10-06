use casbin::CoreApi;
use chrono::NaiveDateTime;
use rocket::State;
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::state::{TiberiusRequestState, TiberiusState};
use tracing::{error, warn};

pub mod channels;
pub mod flash;
pub mod frontmatter;
pub mod image;
pub mod pagination;
pub mod routes;
pub mod streambox;
pub mod renderer;

pub enum APIMethod {
    Create,
    Delete,
    Update,
    View,
    List,
}

pub async fn camoed_url(state: &TiberiusState, url: &url::Url) -> String {
    let conf: &Configuration = &state.config;
    match conf.camo_config() {
        Some((camo_host, camo_key)) => {
            let config = camo_url::CamoConfig::new(hex::encode(camo_key), camo_host);
            match config {
                Err(e) => {
                    error!("error in camo config: {}", e);
                    url.to_string()
                }
                Ok(config) => match config.get_camo_url(&url) {
                    Err(e) => {
                        error!("could not generate camo urls: {}", e);
                        url.to_string()
                    }
                    Ok(url) => url.to_string(),
                },
            }
        }
        None => {
            warn!("no camo key or host configured");
            url.to_string()
        }
    }
}

pub fn pluralize<S: Into<String>>(singular: S, plural: S, count: i32) -> String {
    if count == 1 {
        let singular: String = singular.into();
        format!("{} {}", count, singular)
    } else {
        let plural: String = plural.into();
        format!("{} {}", count, plural)
    }
}

pub fn human_date(d: NaiveDateTime) -> String {
    format!(
        "{}",
        chrono_humanize::HumanTime::from(chrono::DateTime::<chrono::Utc>::from_utc(d, chrono::Utc))
    )
}

#[derive(Clone, PartialEq)]
pub enum ACLSubject {
    None,
    User(tiberius_models::User),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ACLObject {
    Image,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ACLActionImage {
    ChangeUploader,
}

pub trait ACLSubjectTrait {
    fn subject(&self) -> String;
}

pub trait ACLObjectTrait {
    fn object(&self) -> String;
}

pub trait ACLActionTrait {
    fn action(&self) -> String;
}

impl ACLObjectTrait for ACLObject {
    fn object(&self) -> String {
        match self {
            ACLObject::Image => "image".to_string(),
        }
    }
}

impl ACLSubjectTrait for ACLSubject {
    fn subject(&self) -> String {
        match self {
            ACLSubject::User(v) => {
                format!("user::{}", v.id)
            }
            ACLSubject::None => "user::anonymous".to_string(),
        }
    }
}

impl ACLActionTrait for ACLActionImage {
    fn action(&self) -> String {
        match self {
            ACLActionImage::ChangeUploader => "change_uploader".to_string(),
        }
    }
}

pub async fn verify_acl(
    state: &State<TiberiusState>,
    rstate: &TiberiusRequestState<'_>,
    object: impl ACLObjectTrait,
    action: impl ACLActionTrait,
) -> TiberiusResult<bool> {
    let casbin = state.get_casbin();
    let subject = rstate.user(state).await?;
    let subject = match subject {
        None => ACLSubject::None,
        Some(v) => ACLSubject::User(v),
    };
    let v = (subject.subject(), object.object(), action.action());
    debug!("Checking if {:?} is OK in RBAC", v);
    let enforce_result = casbin.enforce(v)?;
    todo!();
}
