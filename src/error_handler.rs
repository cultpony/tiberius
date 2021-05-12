use anyhow::Result;
use tide::Request;

use crate::config::Configuration;

pub struct ErrorHandler {}

impl ErrorHandler {
    pub fn new(c: &Configuration) -> Result<Self> {
        //TODO: allow configuring detailed vs non-detailed errors
        Ok(Self {})
    }
}

#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State> for ErrorHandler {
    async fn handle(&self, req: Request<State>, next: tide::Next<'_, State>) -> tide::Result {
        let mut res = next.run(req).await;
        if let Some(err) = res.error() {
            res.set_body(format!("An internal error has occured: {}", err));
        }
        Ok(res)
    }
}
