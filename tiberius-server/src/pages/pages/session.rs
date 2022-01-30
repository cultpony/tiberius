use maud::{html, Markup, PreEscaped};
use rocket::{form::Form, response::Redirect, State};
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::{HtmlResponse, RedirectResponse, TiberiusResponse};
use tiberius_core::session::{Authenticated, SessionMode, Unauthenticated, AuthMethod};
use tiberius_core::state::{Flash, TiberiusRequestState, TiberiusState};
use tiberius_models::{Client, User, UserLoginResult};

use crate::pages::common::flash::put_flash;

#[get("/sessions/login")]
pub async fn new_session(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let state = state.inner().clone();
    let mut client: Client = state.get_db_client().await?;
    let body = html! {
        h1 { "Sign in" }

        form action=(uri!(new_session_post)) method="POST" {
            //TODO: include flash messages
            //TODO: CSRF protection
            p {
                a href=(uri!(forgot_password)) { "Forgot your password?"}
            }

            .field {
                input.input #user_email name="email" type="email" required="true" placeholder="Email" autofocus="true" pattern=".*@.*";
            }

            .field {
                input.input #user_password name="password" type="password" required="true" placeholder="Password";
            }

            .field {
                input.input #user_totp name="totp" type="text" pattern="[0-9]{6}" placeholder="TOTP";
            }

            /*.field { We don't implement session remembering, just let the session linger
                input#user_remember_me name="remember_me" type="checkbox" value="true";
                label for="user_remember_me" { "Remember me" }
            }*/

            .actions {
                button.button type="submit" { "Sign in" }
            }
        }

        p {
            strong {
                "Haven't read the "
                a href="/pages/rules" { "site rules" }
                " lately? Make sure you read them before posting or editing metadata!"
            }
        }
    };
    let page: PreEscaped<String> = html! {
        (crate::pages::common::frontmatter::app(&state, &rstate, None, &mut client, body, None).await?);
    };
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: page.into_string(),
    }))
}

#[get("/api/v3/sessions/login")]
pub async fn alt_url_new_session(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Unauthenticated>,
) -> TiberiusResult<TiberiusResponse<()>> {
    let state = state.inner().clone();
    let mut client: Client = state.get_db_client().await?;
    let body = html! {
        h1 { "Sign in" }
        h3 { b { "Alternative login page: Ensure you have the v3-Deployment Key setup in your browser"} }

        form action=(uri!(alt_url_new_session_post)) method="POST" {
            input type="hidden" name="alt_r" value="1";

            .field {
                input.input #user_email name="email" type="email" required="true" placeholder="Email" autofocus="true" pattern=".*@.*";
            }

            .field {
                input.input #user_password name="password" type="password" required="true" placeholder="Password";
            }

            .field {
                input.input #user_totp name="totp" type="text" pattern="[0-9]{6}" placeholder="TOTP";
            }

            .actions {
                button.button type="submit" { "Sign in" }
            }
        }

        p {
            strong {
                "Haven't read the "
                a href="/pages/rules" { "site rules" }
                " lately? Make sure you read them before posting or editing metadata!"
            }
        }
    };
    let page: PreEscaped<String> = html! {
        (crate::pages::common::frontmatter::app(&state, &rstate, None, &mut client, body, None).await?);
    };
    Ok(TiberiusResponse::Html(HtmlResponse {
        content: page.into_string(),
    }))
}

#[get("/session/forgot_pw")]
pub async fn forgot_password() -> TiberiusResult<String> {
    todo!()
}

#[derive(FromForm)]
pub struct NewSession<'r> {
    email: &'r str,
    password: &'r str,
    totp: Option<&'r str>,
    // use alternative login route and success
    alt_r: Option<bool>,
}

#[post("/api/v3/sessions/login", data = "<login_data>")]
pub async fn alt_url_new_session_post(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Unauthenticated>,
    login_data: Form<NewSession<'_>>,
) -> TiberiusResult<RedirectResponse> {
    Ok(new_session_post(state, rstate, login_data).await?)
}

#[post("/sessions/login", data = "<login_data>")]
pub async fn new_session_post(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Unauthenticated>,
    login_data: Form<NewSession<'_>>,
) -> TiberiusResult<RedirectResponse> {
    trace!("requesting new session, verifying user");
    let mut client = state.get_db_client().await?;

    let user: Option<User> =
        User::get_mail_or_name(&mut client, login_data.email).await?;
    if let Some(user) = user {
        let valid = user.validate_login(state.config.password_pepper(), &state.config.otp_secret(), login_data.email, login_data.password, login_data.totp)?;
        let home = if login_data.alt_r.unwrap_or(false) {
            uri!(crate::pages::activity::index)
        } else {
            uri!(crate::api::v3::misc::sessho::session_handover_user)
        };
        let retry = if login_data.alt_r.unwrap_or(false) {
            uri!(new_session)
        } else {
            uri!(alt_url_new_session)
        };
        match valid {
            UserLoginResult::Valid => {
                let session = rstate.session;
                session.write().await.set_user(&user);
                let id = session.read().await.id();
                trace!("Creating new session, persisting {} to DB", id);
                session.write().await.save(state).await;
                Ok(RedirectResponse {
                    redirect: Flash::alert("Login successfull!")
                        .into_resp(Redirect::to(home)),
                })
            },
            UserLoginResult::Invalid => {
                trace!("password disagree");
                Ok(RedirectResponse {
                    redirect: Flash::alert("User or password incorrect")
                        .into_resp(Redirect::to(retry)),
                })
            }
            UserLoginResult::RetryWithTOTP => {
                trace!("password agree, TOTP missing");
                Ok(RedirectResponse {
                    redirect: Flash::alert("TOTP incorrect or required")
                        .into_resp(Redirect::to(retry)),
                })
            }
        }
    } else {
        trace!("user not found");
        Ok(RedirectResponse {
            redirect: Flash::alert("User or password incorrect")
                .into_resp(Redirect::to(uri!(new_session))),
        })
    }
}

#[post("/sessions/register")]
pub async fn registration() -> TiberiusResult<String> {
    todo!()
}

#[get("/session/logout")]
pub async fn destroy_session(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Authenticated>,
) -> TiberiusResult<RedirectResponse> {
    let session = rstate.session;
    session.write().await.unset_user();
    session.write().await.save(state).await;
    Ok(RedirectResponse {
        redirect: Flash::info("You have been logged out")
            .into_resp(Redirect::to(uri!(crate::pages::activity::index))),
    })
}

#[get("/api/v3/sessions/logout")]
pub async fn alt_url_destroy_session(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, Authenticated>,
) -> TiberiusResult<RedirectResponse> {
    let session = rstate.session;
    session.write().await.unset_user();
    session.write().await.save(state).await;
    Ok(RedirectResponse {
        redirect: Flash::info("You have been logged out")
            .into_resp(Redirect::to(uri!(alt_url_new_session))),
    })
}
