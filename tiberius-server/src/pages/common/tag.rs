use axum_extra::routing::TypedPath;
use either::Either;
use maud::{html, Markup};
use tiberius_models::{Tag, TagView};

use crate::pages::{
    session::PathNewSession,
    tags::{
        PathTagsByNameShowTag, PathTagsHideTag, PathTagsShowTag, PathTagsSpoilerTag,
        PathTagsWatchTag,
    },
    PathFilters,
};

pub fn tag_markup(tag: &TagView) -> Markup {
    html! {
        span.tag.dropdown data-tag-category=(tag.category.as_ref().unwrap_or(&"".to_string())) data-tag-id=(tag.id) data-tag-name=(tag.name) data-tag-slug=(tag.slug.clone().unwrap_or_default()) {
            span {
                span.tag__state.hidden title="Unwatched" { "+" }
                span.tag__state.hidden title="Watched" { "-" }
                span.tag__state.hidden title="Spoilered" { "S" }
                span.tag__state.hidden title="Hidden" { "H" }
                a class="tag__name" href=(tag.slug.as_ref().map(|slug| PathTagsByNameShowTag{ tag: slug.clone() }.to_uri())
                    .unwrap_or(PathTagsShowTag{ tag_id: tag.id as i64 }.to_uri()))
                    title=(tag.description.clone().unwrap_or(tag.name.clone())) {
                        " " (tag.name)
                }
            }
            div.dropdown__content {
                a.tag__dropdown__link data-method="delete" data-tag-action="unwatch" href=(PathTagsWatchTag{ tag_id: tag.id as u64 }.to_uri()) { "Unwatch" }
                a.tag__dropdown__link data-method="post" data-tag-action="unwatch" href=(PathTagsWatchTag{ tag_id: tag.id as u64 }.to_uri()) { "Watch" }

                a.tag__dropdown__link data-method="delete" data-tag-action="unspoiler" href=(PathTagsSpoilerTag{ tag_id: tag.id as u64 }.to_uri()) { "Unspoiler" }
                a.tag__dropdown__link data-method="post" data-tag-action="spoiler" href=(PathTagsSpoilerTag{ tag_id: tag.id as u64 }.to_uri()) { "Spoiler" }

                a.tag__dropdown__link data-method="delete" data-tag-action="unhide" href=(PathTagsHideTag{ tag_id: tag.id as u64 }.to_uri()) { "Unhide" }
                a.tag__dropdown__link data-method="post" data-tag-action="hide" href=(PathTagsHideTag{ tag_id: tag.id as u64 }.to_uri()) { "Hide" }

                a.tag__dropdown__link href=(PathNewSession{}.to_uri()) { "Sign in to Watch" }
                a.tag__dropdown__link href=(PathFilters{}.to_uri()) { "Filters" }
            }
            span.tag__count {
                (tag.images_count)
            }
        }
    }
}
