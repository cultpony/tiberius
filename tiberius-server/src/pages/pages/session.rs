use maud::{html, Markup, PreEscaped};
use rocket::{form::Form, response::Redirect, State};
use tiberius_core::error::TiberiusResult;
use tiberius_core::request_helper::{HtmlResponse, RedirectResponse, TiberiusResponse};
use tiberius_core::session::SessionMode;
use tiberius_core::state::{Flash, TiberiusRequestState, TiberiusState};
use tiberius_models::{Client, User};

use crate::pages::common::flash::put_flash;

#[get("/sessions/login")]
pub async fn new_session(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, {SessionMode::Unauthenticated}>,
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

#[get("/session/forgot_pw")]
pub async fn forgot_password() -> TiberiusResult<String> {
    todo!()
}

#[derive(FromForm)]
pub struct NewSession<'r> {
    email: &'r str,
    password: &'r str,
}

#[post("/sessions/login", data = "<login_data>")]
pub async fn new_session_post(
    state: &State<TiberiusState>,
    rstate: TiberiusRequestState<'_, {SessionMode::Unauthenticated}>,
    login_data: Form<NewSession<'_>>,
) -> TiberiusResult<RedirectResponse> {
    trace!("requesting new session, verifying user");
    let mut client = state.get_db_client().await?;

    let user: Option<User> =
        User::get_mail_or_name(&mut client, login_data.email.to_string()).await?;
    if let Some(user) = user {
        let password = login_data.password.to_string()
            + state
                .config
                .password_pepper
                .as_ref()
                .unwrap_or(&"".to_string())
                .as_str();
        let hash = &user.encrypted_password;
        trace!("password: {}, hash: {}", password, hash);
        let valid = bcrypt::verify(password, &hash)?;
        if valid {
            let session = rstate.session;
            session.write().await.set_user(&user);
            let id = session.read().await.id();
            trace!("Creating new session, persisting {} to DB", id);
            session.write().await.save(state).await;
            Ok(RedirectResponse {
                redirect: Flash::alert("Login successfull!")
                    .into_resp(Redirect::to(uri!(crate::pages::activity::index))),
            })
        } else {
            trace!("password disagree");
            Ok(RedirectResponse {
                redirect: Flash::alert("User or password incorrect")
                    .into_resp(Redirect::to(uri!(new_session))),
            })
        }
    } else {
        trace!("user not found");
        Ok(RedirectResponse {
            redirect: Flash::alert("User or password incorrect")
                .into_resp(Redirect::to(uri!(new_session))),
        })
    }
}

#[post("/sessions/login/totp")]
pub async fn new_session_totp() -> TiberiusResult<String> {
    todo!()
}

#[post("/sessions/register")]
pub async fn registration() -> TiberiusResult<String> {
    todo!()
}

#[get("/session/logout")]
pub async fn destroy_session(rstate: TiberiusRequestState<'_, {SessionMode::Authenticated}>) -> TiberiusResult<RedirectResponse> {
    Ok(RedirectResponse {
        redirect: Flash::info("You have been logged out")
            .into_resp(Redirect::to(uri!(crate::pages::activity::index))),
    })
}
