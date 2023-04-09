use maud::html;

use crate::email::SEPARATOR;

const VISIT_URL: &str = "You can reset your password by visiting the URL below:";
const GREETING: &str = "Hi ";
const IGNORE_REQ: &str = "If you didn't request this change, please ignore this.";
const SUBJECT: &str = "Password reset instructions for your account";

pub fn subject(user_name: &str) -> String {
    format!("{SUBJECT} '{user_name}'")
}

pub fn build(user_name: &str, reset_url: &str) -> maud::Markup {
    html! {
        p {
            (SEPARATOR)
        }
        p { b { (GREETING) (user_name) "," } }
        p {
            (VISIT_URL)
        }
        p {
            a href=(reset_url) { (reset_url) }
        }
        p {
            (IGNORE_REQ)
        }
        p {
            (SEPARATOR)
        }
    }
}

pub fn build_txt(user_name: &str, reset_url: &str) -> String {
    format!(r#"
{SEPARATOR}

{GREETING} {user_name},

{VISIT_URL}

{reset_url}

{IGNORE_REQ}

{SEPARATOR}
"#)
}