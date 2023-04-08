#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct UserSettings {
    pub spoiler_type: String,
    pub theme: String,
    pub images_per_page: i32,

    pub show_large_thumbnails: bool,
    pub show_sidebar_and_watched_images: bool,

    pub fancy_tag_field_on_upload: bool,
    pub fancy_tag_field_on_edit: bool,
    pub fancy_tag_field_in_settings: bool,

    pub autorefresh_by_default: bool,
    pub anonymous_by_default: bool,
    pub scale_large_images: bool,

    pub comments_newest_first: bool,
    pub comments_always_jump_to_last: bool,
    pub comments_per_page: i32,

    pub watch_on_reply: bool,
    pub watch_on_new_topic: bool,
    pub watch_on_upload: bool,

    pub messages_newest_first: bool,

    pub serve_webm: bool,
    pub no_spoilered_in_watched: bool,
    pub watched_images_query_str: String,
    pub watched_images_exclude_str: String,
    pub recent_filter_ids: Vec<i32>,
    pub watched_tag_ids: Vec<i32>,
    pub current_filter_id: Option<i32>,

    pub use_centered_layout: bool,
    pub forced_filter_id: Option<i64>,
    pub show_hidden_items: bool,
    pub hide_vote_counts: bool,
    pub hide_advertisements: bool,
    pub hide_default_role: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            spoiler_type: String::default(),
            theme: String::default(),
            images_per_page: 20,

            show_large_thumbnails: false,
            show_sidebar_and_watched_images: false,

            fancy_tag_field_on_upload: true,
            fancy_tag_field_on_edit: true,
            fancy_tag_field_in_settings: true,

            autorefresh_by_default: false,
            anonymous_by_default: false,
            scale_large_images: true,

            comments_newest_first: true,
            comments_always_jump_to_last: false,
            comments_per_page: 20,

            watch_on_reply: true,
            watch_on_new_topic: true,
            watch_on_upload: true,

            messages_newest_first: true,

            serve_webm: true,
            no_spoilered_in_watched: true,
            watched_images_query_str: String::default(),
            watched_images_exclude_str: String::default(),
            recent_filter_ids: Vec::new(),
            watched_tag_ids: Vec::new(),
            current_filter_id: None,

            use_centered_layout: false,
            forced_filter_id: None,
            show_hidden_items: false,
            hide_vote_counts: false,
            hide_advertisements: false,
            hide_default_role: false,
        }
    }
}
