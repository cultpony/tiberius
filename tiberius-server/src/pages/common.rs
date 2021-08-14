use chrono::NaiveDateTime;
use tiberius_core::config::Configuration;
use tiberius_core::state::TiberiusState;
use tracing::{error, warn};

pub mod channels;
pub mod flash;
pub mod frontmatter;
pub mod image;
pub mod pagination;
pub mod routes;
pub mod streambox;

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
