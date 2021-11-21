use std::collections::BTreeMap;

use either::Either;

use crate::error::TiberiusResult;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct FooterData {
    pub cols: Vec<String>,
    #[serde(flatten)]
    pub rows: BTreeMap<String, Vec<FooterRow>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct FooterRow {
    pub title: String,
    #[serde(with = "either::serde_untagged")]
    pub url: Either<url::Url, std::path::PathBuf>,
    #[serde(default)]
    pub bold: bool,
}

impl FooterRow {
    pub fn url(&self) -> TiberiusResult<String> {
        match &self.url {
            Either::Left(url) => Ok(url.to_string()),
            Either::Right(path) => Ok(path.to_string_lossy().to_string()),
        }
    }
}
