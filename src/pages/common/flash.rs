use crate::app::HTTPReq;

use anyhow::Result;
use log::trace;

pub enum Flash {
    Info(String),
    Alert(String),
    Error(String),
    Warning(String),
    None,
}

pub fn get_flash(_req: &HTTPReq) -> Result<Flash> {
    trace!("loading flash notices from session");
    Ok(Flash::None)
}

pub fn put_flash(_req: &HTTPReq, _f: Flash) -> Result<()> {
    trace!("putting flash into session");
    todo!("cannot store flash in session yet")
}
