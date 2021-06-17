use crate::{app::HTTPReq, error::TiberiusResult};

use log::trace;

#[derive(serde::Deserialize, serde::Serialize)]
pub enum Flash {
    Info(String),
    Alert(String),
    Error(String),
    Warning(String),
    None,
}

impl Flash {
    pub fn error<S: Into<String>>(e: S) -> Flash {
        Self::Error(e.into())
    }
    pub fn alert<S: Into<String>>(a: S) -> Flash {
        Self::Alert(a.into())
    }
    pub fn warning<S: Into<String>>(w: S) -> Flash {
        Self::Warning(w.into())
    }
    pub fn info<S: Into<String>>(i: S) -> Flash {
        Self::Info(i.into())
    }
}

impl Default for Flash {
    fn default() -> Self {
        Self::None
    }
}

pub fn get_flash(req: &mut HTTPReq) -> TiberiusResult<Vec<Flash>> {
    trace!("loading flash notices from session");
    let flashlist = req.session().get::<Vec<Flash>>("flash");
    req.session_mut().remove("flash");
    Ok(flashlist.unwrap_or_default())
}

pub fn put_flash(req: &mut HTTPReq, f: Flash) -> TiberiusResult<()> {
    trace!("putting flash into session");
    let flashlist = req.session_mut().get::<Vec<Flash>>("flash");
    if let Some(mut flashlist) = flashlist {
        flashlist.push(f);
        req.session_mut().insert("flash", flashlist);
    } else {
        req.session_mut().insert("flash", vec![f]);
    }
    Ok(())
}
