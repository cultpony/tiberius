use maud::Markup;

use crate::{
    app::HTTPReq,
    pages::{todo_page, ResToResponse},
};

pub async fn list_tags() -> Markup {
    todo_page("list_tags").await
}

pub async fn show_tag() -> Markup {
    todo_page("show_tags").await
}

pub async fn edit_tag() -> Markup {
    todo_page("edit_tags").await
}

pub async fn tag_changes() -> Markup {
    todo_page("tag_tags").await
}

pub async fn usage() -> Markup {
    todo_page("usage_tags").await
}
