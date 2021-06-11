use std::{path::PathBuf, str::FromStr};

use crate::app::HTTPReq;
use log::trace;
use new_mime_guess::Mime;

pub mod common;
mod pages;
pub mod views;
pub use pages::*;
mod files;
pub use files::*;
