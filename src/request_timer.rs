use std::time::Instant;

use rocket::fairing::{Fairing, Info, Kind};
use tide::Request;

use crate::state::State;

pub struct RequestTimer();

pub struct RequestStartInstant(pub Instant);

#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> tide::Middleware<State> for RequestTimer {
    async fn handle(&self, mut req: Request<State>, next: tide::Next<'_, State>) -> tide::Result {
        let start = Instant::now();
        req.set_ext(RequestStartInstant(start));
        let mut res = next.run(req).await;
        let time_taken = Instant::now().duration_since(start);
        res.insert_header(
            "x-time-taken",
            format!("{:1.3}ms", time_taken.as_secs_f32() * 1000.0),
        );

        Ok(res)
    }
}

#[rocket::async_trait]
impl Fairing for RequestTimer {
    fn info(&self) -> Info {
        Info {
            name: "Request Timer",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        request.
    }
}

pub trait RequestTimerRequestExt {
    fn expired_duration(&self) -> std::time::Duration {
        Instant::now().duration_since(self.start_instant())
    }
    fn start_instant(&self) -> std::time::Instant;
    fn expired_time_ms(&self) -> f32 {
        self.expired_duration().as_secs_f32() * 1000.0
    }
}

impl RequestTimerRequestExt for Request<State> {
    fn start_instant(&self) -> std::time::Instant {
        let start = self
            .ext::<RequestStartInstant>()
            .expect("Request timer not in connection");
        start.0
    }
}
