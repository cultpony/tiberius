use maud::html;

use crate::email::SEPARATOR;

const VISIT_URL: &str = "You can unlock your account by visting the URL below:";
const GREETING: &str = "Hi ";
const ACC_LOCKED: &str =
    "Your account has been automatically locked due to too many attempts to sign in.";
const SUBJECT: &str = "Unlock instructions for your account";

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
            (ACC_LOCKED)
        }
        p {
            (VISIT_URL)
        }
        p {
            a href=(reset_url) { (reset_url) }
        }
        p {
            (SEPARATOR)
        }
    }
}

pub fn build_txt(user_name: &str, reset_url: &str) -> String {
    format!(
        r#"
{SEPARATOR}

{GREETING} {user_name},

{ACC_LOCKED}

{VISIT_URL}

{reset_url}

{SEPARATOR}
"#
    )
}
