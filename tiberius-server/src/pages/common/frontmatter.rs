use itertools::Itertools;
use std::fmt::Display;
use std::{
    collections::BTreeMap,
    fmt::{write, Debug},
};
use tiberius_core::app::PageTitle;
use tiberius_core::assets::{QuickTagTableContent, SiteConfig};
use tiberius_core::error::TiberiusResult;
use tiberius_core::session::Session;
use tiberius_core::state::{SiteNotices, TiberiusRequestState, TiberiusState};

use crate::pages::common::image::image_thumb_urls;
use crate::pages::common::{
    flash::get_flash,
    routes::{cdn_host, dark_stylesheet_path, static_path, stylesheet_path, thumb_url},
};
use either::Either;
use maud::{html, Markup, PreEscaped};
use rocket::Request;
use rocket::{request::FromRequest, uri, State};
use tiberius_models::{
    Channel, Client, Conversation, Filter, Forum, Image, ImageThumbType, Notification, SiteNotice,
    Tag, User,
};
use tracing::trace;

pub fn viewport_meta_tags(rstate: &TiberiusRequestState<'_>) -> Markup {
    let mobile_uas = ["Mobile", "webOS"];
    if let Some(value) = rstate
        .headers
        .get_one(rocket::http::hyper::header::USER_AGENT.as_str())
    {
        for mobile_ua in &mobile_uas {
            if value.to_string().contains(mobile_ua) {
                return html! { meta name="viewport" content="width=device-width, initial-scale=1"; };
            }
        }
    }
    return html! { meta name="viewport" content="width=1024, initial-scale=1"; };
}

pub async fn csrf_meta_tag(rstate: &TiberiusRequestState<'_>) -> Markup {
    let session: &Session = &rstate.session;
    let csrf = session.csrf_token();
    html! {
        meta content=(csrf) csrf-param="_csrf_token" method-param="_method" name="csrf-token";
    }
}

pub fn no_avatar_svg() -> Markup {
    html! {
        svg xmlns="http://www.w3.org/2000/svg" width="125" height="125" viewBox="0 0 125 125" class="avatar-svg" {
            rect width="125" height="125" fill="#c6dff2" {}
            path d="M15.456 109.15C12.02 97.805 6.44 95.036-.794 98.89v19.102c5.13-10.09 10.263-8.294 15.395-5.7" fill="#A29859" {}
            path d="M73.054 24.46c25.886 0 39.144 26.39 28.916 44.95 1.263.38 4.924 2.274 3.41 4.8-1.516 2.525-7.577 16.288-27.78 14.773-1.01 6.44-.33 12.613 1.642 22.854 1.39 7.224-.632 14.648-.632 14.648s-47.785.216-73.74-.127c-1.883-6.387 8.964-25.76 20.833-24.748 15.674 1.334 19.193 1.64 21.592-2.02 2.4-3.662 0-23.234-3.535-30.81-3.536-7.577-7.83-40.785 29.294-44.32z" fill="#4CA782" {}
            path d="M64.335 34.675c3.358 1.584 6.716.908 10.073 1.043-.265 13.078 19.05 19.74 31.58 4.16 6.077 6.273 24.776 2.28 12.42-18.66-12.88-21.833-42.605-11.287-61-.5l-7.25 11c-29.918 14.92-16.418 45.666-.75 57.625-12.967 2.522-6.234 30.16 9.904 24.894 18.84-6.147-1.986-51.066-7.78-62.644l1.495-11.736z" fill="#A29859" {}
            path d="M43.267 107.324s-6.825-14.137-7.64-30.166c-.817-16.03-4.197-31.468-10.55-40.688-6.354-9.22-13.272-9.73-11.997-3.982 1.275 5.748 11.123 33.016 12.128 35.954C23.042 65.648 7.038 41.11-.43 37.222c-7.47-3.886-8.96.346-6.892 5.885 2.068 5.54 18.507 30.844 20.886 33.502-2.738-1.685-12.256-9.036-16.997-8.996-4.742.04-4.91 5.366-2.617 8.526 2.292 3.162 20.912 19.173 25.15 20.945-5.35.28-10.384 1.996-9.186 6.004 1.2 4.006 11.384 14.063 28.53 12.377 2.576-2.834 4.823-8.143 4.823-8.143z" fill="#4CA782" {}
            path d="M64.342 35.57s3.283-8.08-7.324-19.318c-1.768-1.768-3.03-2.273-4.672-.758-1.64 1.515-17.046 16.036.253 38.26.504-2.4 1.135-9.597 1.135-9.597z" fill="#4CA782" {}
        }
    }
}

pub async fn open_graph(state: &TiberiusState, image: Option<Image>) -> TiberiusResult<Markup> {
    let mut client = state.get_db_client().await?;
    let filtered = !image
        .as_ref()
        .map(|x| x.thumbnails_generated)
        .unwrap_or(false);
    let description = image
        .as_ref()
        .map(|img| {
            format!(
                "{} - {} - Manebooru",
                img.id,
                img.tag_list_cache
                    .as_ref()
                    .map(|x| x.as_str())
                    .unwrap_or("")
            )
        })
        .unwrap_or("# - # - Manebooru".to_string());
    Ok(html! {
        meta name="generator" content="tiberius";
        meta name="theme-color" content="#618fc3";
        meta name="format-detection" content="telephone=no";
        @if let Some(image) = image {
            meta name="keywords" content=(image.tag_list_cache.as_ref().map(|x| x.as_str()).unwrap_or(""));
            meta name="description" content=(description);
            meta property="og:title" content=(description);
            meta property="og:url" content=(uri!(crate::pages::images::show_image(image = image.id as u64)).to_string());

            @for tag in artist_tags(&image.tags(&mut client).await?) {
                meta property="dc:creator" content=(tag.full_name());
            }

            @if let Some(source_url) = &image.source_url {
                @if !source_url.is_empty() {
                    meta property="foaf:primaryTopic" content=(source_url);
                }
            }

            link rel="alternate" type="application/json-oembed" href=(uri!(crate::api::int::oembed::fetch)) title="oEmbed JSON Profile";
            link rel="canonical" href=(uri!(crate::pages::images::show_image(image = image.id as u64)));

            @match (image.image_mime_type.as_ref().map(|x| x.as_str()), filtered) {
                (Some("video/webm"), false) => {
                    meta property="og:type" content="video.other";
                    meta property="og:image" content=(uri!(crate::pages::files::image_thumb_get_simple(id = image.id as u64, thumbtype = "rendered", _filename = image.filetypef("rendered"))));
                    meta property="og:video" content=(uri!(crate::pages::files::image_thumb_get_simple(id = image.id as u64, thumbtype = "large", _filename = image.filetypef("large"))));
                },
                (Some("image/svg+xml"), false) => {
                    meta property="og:type" content="website";
                    meta property="og:image" content=(uri!(crate::pages::files::image_thumb_get_simple(id = image.id as u64, thumbtype = "rendered", _filename = image.filetypef("rendered"))));
                },
                (_, false) => {
                    meta property="og:type" content="website";
                    meta property="og:image" content=(uri!(crate::pages::files::image_thumb_get_simple(id = image.id as u64, thumbtype = "large", _filename=image.filename())));
                },
                _ => { meta property="og:type" content="website"; },
            }
        } @else {
            meta name="description" content="Manebooru is a linear imagebooru which lets you share, find and discover new art and media surrounding the show My Little Pony: Friendship is Magic";
        }
    })
}

pub fn artist_tags(tags: &[Tag]) -> Vec<&Tag> {
    tags.iter()
        .filter(|t| {
            t.namespace
                .as_ref()
                .map(|x| x.as_str() == "artist")
                .unwrap_or(false)
        })
        .collect()
}

pub fn burger() -> Markup {
    html! {
        nav#burger {
            a href="/" { i.fa-fw.favicon-home {} "Home" }
            a href="/images/new" { i.fa.fa-fw.fa-upload {} "Upload" }
            a href="/forums" { i.fas.fa-fw.fa-pen-square {} "Forums" }
            a href="/tags" { i.fa.fa-fw.fa-tag {} "Tags" }
            a href="/search?q=first_seen_at.gt:10+minutes+ago&amp;sf=wilson_score&amp;sd=desc" { i.fas.fa-fw.fa-poll {} "Rankings" }
            a href="/filters" { i.fa.fa-fw.fa-filter {} "Filters" }
            a href="/galleries" { i.fa.fa-fw.fa-image {} "Galleries" }
            a href="/comments" { i.fa.fa-fw.fa-comments {} "Comments" }
            a href="/commissions" { i.fa.fa-fw.fa-address-card {} "Commissions" }
            a href="/channels" { i.fa-fw.fa-podcasts {} "Channels" }
            a href="/pages/donations" { i.fa.fa-fw.fa-heart {} "Donate" }
        }
    }
}

pub fn tag_editor<S1: Display, S2: Display>(editor_type: S1, name: S2) -> Markup {
    let ta_class = format!("js-taginput-{}", name);
    html! {
        .js-tag-block.(format!("fancy-tag-{}", editor_type)) {
            textarea.input.input--wide.tagsinput.js-image-input.js-taginput.js-taginput-plain.hidden#image_tag_input.(ta_class) autocomplete="off" name="image.tag_input" placeholder="Add tags seperated with commas" {}
            .js-taginput.input.input--wide.tagsinput.js-taginput-fancy data-click-focus=(format!(".js-taginput-input.js-taginput-{}", name)) {
                input.input.js-taginput-input.(format!("js-taginput-{}", name))#(format!("taginput-fancy-{}", name)) type="text" placeholder="add a tag" autocomplete="off" autocapitalize="none" data-ac="true" data-ac-min-length="3" data-ac-source="/tags/autocomplete?term=" {}
            }
        }
        button.button.button--state-primary.button--bold.js-taginput-show.hidden data-click-show=".js-taginput-fancy,.js-taginput-hide" data-click-hide=".js-taginput-plain,.js-taginput-show" data-click-focus=(format!(".js-taginput.js-taginput-{}", name)) {
            input type="hidden" name="fuck_ie" id="fuck_ie" value="fuck_ie" {}
            "Fancy Editor"
        }
        button.button.button--state-primary.button--bold.js-taginput-hide data-click-show=".js-taginput-plain,.js-taginput-show" data-click-hide=".js-taginput-fancy,.js-taginput-hide" data-click-focus=(format!(".js-taginput-plain.js-taginput-{}", name)) {
            "Plain Editor"
        }
        button.button.button--state-success.button--separate-left.button--bold#tagsinput-save title="This button saves the tags listed above to your browser, allowing you to retrieve them again by clicking the Load button" {
            "Save"
        }
        button.button.button--state-warning.button--separate-left.button--bold#tagsinput-save title="This button loads any saved tags from your browser" {
            "Load"
        }
        button.button.button--state-danger.button--separate-left.button--bold#tagsinput-clear title="This button will clear the list of tags above" type="button" {
            "Clear"
        }
    }
}

pub fn tag_link(uri: bool, tag: &str, name: &str) -> Markup {
    //TODO: set proper title for tag description
    let uri = if uri {
        uri!(crate::pages::tags::show_tag_by_name(tag = tag)).to_string()
    } else {
        "#".to_string()
    };
    html! {
        a href=(uri) data-tag-name=(tag) data-click-addtag=(tag) { (name) }
    }
}

pub fn quick_tag_table(state: &TiberiusState) -> Markup {
    let asset_loader = &state.asset_loader;
    let qtt = asset_loader.quick_tag_table();
    let mut qtt_tabs_content = Vec::new();
    let mut qtt_tabs = Vec::new();
    for (i, qtte) in qtt.iter().enumerate() {
        qtt_tabs.push(html! {
            a href="#" data-click-tab=(qtte.title) { (qtte.title) }
        });
        let body_class = match i {
            0 => "",
            _ => "hidden",
        };
        qtt_tabs_content.push(html! {
            .block__tab.quick-tag-table__tab.(body_class) data-tab=(qtte.title) {
                @match &qtte.content {
                    QuickTagTableContent::Default(d) => {
                        @for table in &d.tables {
                            div {
                                strong { (table.title) }
                                @for tag_name in &table.tags {
                                    br;
                                    (tag_link(false, &tag_name, &tag_name))
                                }
                            }
                        }
                    },
                    QuickTagTableContent::ShortHand(sh) => {
                        @for mapping in &sh.mappings {
                            div {
                                strong{ (mapping.title) }
                                @for (name, alias_name) in mapping.map.iter() {
                                    br;
                                    (name)
                                    " - "
                                    (tag_link(false, &alias_name, &alias_name))
                                }
                            }
                        }
                    },
                    QuickTagTableContent::Shipping(sp) => {
                        //TODO: figure out how to bring up shipping tags automatically
                    },
                    QuickTagTableContent::Season(se) => {
                        @for episode_chunk in &se.episodes.as_slice().into_iter().chunks(10) {
                            div {
                                @for episode in episode_chunk {
                                    (episode.episode_number)
                                    ". "
                                    (tag_link(false, &episode.name, &episode.name))
                                    br;
                                }
                            }
                        }
                    },
                }
            }
        });
    }
    html! {
        .block__header--sub.block__header--js-tabbed {
            @for qtter in qtt_tabs {
                (qtter)
            }
        }
        @for qtter in qtt_tabs_content {
            (qtter)
        }
        br;
    }
}

pub async fn header(
    site_config: &SiteConfig,
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
) -> TiberiusResult<Markup> {
    let notifications = rstate.notifications().await?;
    let mut client = state.get_db_client().await?;
    let filter: Filter = rstate.filter(state).await?;
    trace!("preloading data for header html");
    let user = rstate.user(state).await?;
    let conversations = rstate.conversations().await?;
    trace!("generating header html");
    Ok(html! {
        header.header {
            .flex.flex--centered.flex--start-bunched.flex--maybe-wrap {
                .flex.flex--centered {
                    #js-burger-toggle.hide-desktop {
                        a.header__link href="#" {
                            i.fa.fa-bars {}
                        }
                    }
                    a.header__link href="/" {
                        i.fa.fw.favicon-home {}
                        span.fa__text.hide-limited-desktop.hide_mobile { (site_config.site_name()) }
                    }
                    a.header__link.hide_mobile href="/images/new" title="Upload" {
                        i.fa.fa-upload {}
                    }
                }

                form.header__search.flex.flex--nowrap.flex--centered.hform action=(uri!(crate::pages::images::search_empty)) method="GET" {
                    input.input.header__input.header__input--search#q name="q" title="For terms all required, separate with ',' or 'AND'; also supports 'OR' for optional terms and '-' or 'NOT' for negation. Search with a blank query for more options or click the ? for syntax help."
                        value=(rstate.search_query().await?.to_string()) placeholder="Search" autocapitalize="none";

                    //TODO: sf+sd params https://github.com/derpibooru/philomena/blob/355ce491accae4702f273334271813e93a261e0f/lib/philomena_web/templates/layout/_header.html.slime#L17

                    //TODO: hides_images https://github.com/derpibooru/philomena/blob/355ce491accae4702f273334271813e93a261e0f/lib/philomena_web/templates/layout/_header.html.slime#L22

                    button.header__search__button type="submit" title="Search" {
                        i.fa-embedded--search {}
                    }
                    a.header__search__button href="/search/reverse" title="Search using an image" {
                        i.fa-embedded--camera {}
                    }
                    a.header__search__button href="/pages/search_syntax" title="Search syntax help" {
                        i.fa-embedded--help {}
                    }
                }

                .flex.flex--centered.flex--no-wrap.header__force-right {
                    @if let Some(user) = &user {
                        a.header__link href="/notifications" title="Notification" {
                            i.fa-embedded-notification { }
                            span.js-notification-ticker.fa__text.header__counter data-notification-count=(notifications.len());
                        }

                        a.header__link href="/conversations" title="Conversations" {
                            @if conversations.len() > 0 {
                                i.fa-embedded-unread-message { }
                                span.fa-embedded__text.header__counter {
                                    (conversations.len());
                                }
                            } @else {
                                i.fa-embedded-message { }
                                span.fa-embedded__text.header__counter {
                                    "0";
                                }
                            }
                        }

                        a.header__link.hide-mobile href="/filters" title="Filters" {
                            i.fa.fa-filter {}
                            span.hide-limited-desktop { "Filters"; }
                        }

                        // TODO: user change filter form https://github.com/derpibooru/philomena/blob/355ce491accae4702f273334271813e93a261e0f/lib/philomena_web/templates/layout/_header.html.slime#L52
                        form#filter-quick-form.header__filter-form action="// TODO: filter form" method="POST" {}

                        // TODO: user change hide/spoiler form https://github.com/derpibooru/philomena/blob/355ce491accae4702f273334271813e93a261e0f/lib/philomena_web/templates/layout/_header.html.slime#L55
                        form#spoiler-quick-form.header__filter-form.hide-mobile.hide-limited-desktop action="// TODO: quick spoiler form" method="POST" {}


                        .dropdown.header_dropdown {
                            a.header__link.header__link-user href=(uri!(crate::pages::session::registration)) {
                                //TODO: render user attribution view
                                .image-constrained."avatar--28px" {
                                    (no_avatar_svg())
                                }
                                span.header__link-user__dropdown__content.hide-mobile data-click-preventdefault="true";
                            }
                            nav.dropdown__content.dropdown__content-right.hide-mobile.js-burger-links {
                                a.header__link href=(uri!(crate::pages::session::registration)) { (user.name); }
                                a.header__link href="/search?q=my:watched" { i.fa.fa-fw.fa-eye { "Watched"; } }
                                a.header__link href="/search?q=my:faves" { i.fa.fa-fw.fa-start { "Faves"; } }
                                a.header__link href="/search?q=my:upvotes" { i.fa.fa-fw.fa-arrow-up { "Upvotes"; } }
                                a.header__link href=(uri!(crate::pages::session::registration)) { i.fa.fa-fw.fa-image { "Galleries"; }}
                                a.header__link href="/search?q=my:uploads" { i.fa.fa-fw.fa-upload { "Uploads"; } }
                                a.header__link href="/comments?cq=my:comments" { i.fa.fa-fw.fa-comments { "Comments"; } }
                                a.header__link href="/posts?pq=my:watched" { i.fa.fa-fw.fa-pen-square { "Posts"; } }
                                a.header__link href=(uri!(crate::pages::session::registration)) { i.fa.fa-fw.fa-link { "Links"; } }
                                a.header__link href="/settings/edit" { i.fa.fa-fw.fa-cogs { "Settings"; } }
                                a.header__link href="/conversations" { i.fa.fa-fw.fa-envelope { "Messages"; } }
                                a.header__link href=(uri!(crate::pages::session::registration)) { i.fa.fa-fw.fa-user { "Account"; } }
                                a.header__link href=(uri!(crate::pages::session::destroy_session)) { i.fa.fa-fw.fa-sign-out-alt { "Logout"; } }
                            }
                        }
                    } @else {
                        a.header__link.hide-mobile href="/filters" { (format!("Filters ({})", filter.name)) }
                        span.js-burger-links.hide-mobile {
                            a.header__link href="/settings/edit" {
                                i.fa.fa-fw.fa-cogs.hide-desktop { "Settings" }
                            }
                        }
                        a.header__link href=(uri!(crate::pages::session::registration)) { "Register" }
                        a.header__link href=(uri!(crate::pages::session::new_session)) { "Login" }
                    }
                }
            }
        }
        nav.header.header--secondary {
            .flex.flex--centered.flex--spaced-out.flex--wrap {
                (header_navigation_links(&mut client).await?)
                @if user.map(|x| x.role) != Some("user".to_string()) {
                    (header_staff_links())
                }
            }
        }
    })
}

pub async fn header_navigation_links<'a>(client: &mut Client) -> TiberiusResult<Markup> {
    trace!("generating header_nav links");
    Ok(html! {
        .hide-mobile {
            .dropdown.header__dropdown {
                a.header__link href="/images" {
                    "Images ";
                    span data-click-preventdefault="true" {
                        i.fa.fa-caret-down {}
                    }
                }
                .dropdown__content {
                    a.header__link href="/images/random" { "Random" }
                }
            }
            .dropdown.header__dropdown {
                a.header__link href="/activity" {
                    "Activity ";
                    span data-click-preventdefault="true" {
                        i.fa.fa-caret-down {}
                    }
                }
                .dropdown__content {
                    a.header__link href="/comments" {
                        "Comments"
                    }
                }
            }
            .dropdown.header__dropdown {
                a.header__link href="/forums" {
                    "Forums ";
                    span data-click-preventdefault="true" {
                        i.fa.fa-caret-down {}
                    }
                }
                .dropdown__content {
                    @for forum in Forum::all(client).await? {
                        a.header__link href=(uri!(crate::pages::session::registration)) {
                            (forum.name)
                        }
                    }
                    a.header__link href="/posts" {
                        i.fa.fa-fw.fa-search {
                            "Post Search "
                        }
                    }
                }
            }
            a.header__link href="/tags" { "Tags " }
            a.header__link href="/channels" { "Live " span.header__counter { (Channel::get_live_count(client).await?) } }
            a.header__link href="/galleries" { "Galleries " }
            a.header__link href="/commissions" { "Commissions " }
        }
    })
}

pub fn header_staff_links() -> Markup {
    html! {
        .flex.flex--cenetered.header--secondary__admin-links.stretched-mobile-links.js-staff-action {
            //TODO: add staff links
        }
    }
}

pub async fn flash_warnings(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
) -> TiberiusResult<Markup> {
    let site_notices: Option<SiteNotices> = state.site_notices();
    let site_notices = site_notices.unwrap_or_default();
    use tiberius_core::state::Flash;
    let flash_body = html! {
        @for flash in get_flash(state, rstate).await? {
            @match flash {
                Flash::Info(text) => { .flash.flash--success { (text) } }
                Flash::Alert(text) => { .flash.flash--warning { (text) } }
                Flash::Error(text) => { .flash.flash--warning { (text) } }
                Flash::Warning(text) => { .flash.flash--warning { (text) } }
                Flash::None => {},
            }
        }
    };
    let flash_pre = html! {
        @for notice in site_notices.0 {
            .flash.flash--site-notice {
                strong { (notice.title); }
                " "
                (notice.text)
                " "
                @match &notice.link {
                    Some(link) => {
                        a href=(link) {
                            (notice.link_text.as_ref().unwrap_or(link))
                        }
                    },
                    None => {},
                }
            }
        }
    };
    let noscript = html! {
        noscript.flash.flash--warning {
            strong { "You don't appear to have Javascript enabled"; " " }
            "If you're using an add-on like NoScript, please allow "; " "
            (cdn_host(state, rstate).await); " "
            " for the site to work properly." " ";
        }
    };
    Ok(html! {
        (flash_pre)
        (noscript)
        (flash_body)
    })
}

pub async fn layout_class(req: &TiberiusRequestState<'_>) -> String {
    req.layout_class().await.to_string()
}

pub async fn footer(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
) -> TiberiusResult<Markup> {
    let end_time = rstate.started_at;
    let time = end_time.elapsed();
    let time: f32 = time.as_secs_f32() * 1000f32; // TODO: reimplement measuring this
    let footer_data = state.footer_data();
    let site_config = state.site_config();
    Ok(html! {
        footer#footer {
            div#footer_content {
                @for column in &footer_data.cols {
                    .footercol {
                        h5 { (column) }
                        @for row in &footer_data.rows[column] {
                            @if row.bold {
                                strong { a href=(row.url()?) { (row.title) } }
                            } @else {
                                a href=(row.url()?) { (row.title) }
                            }
                            br;
                        }
                    }
                }
            }
            div#serving_info {
                "Powered by "
                a href=(site_config.source_repo()) { (site_config.source_name()) }
                (format!(" (rendered in {:1.3} ms)", time))
            }
        }
    })
}

pub async fn ignored_tag_list<'a>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
) -> TiberiusResult<Vec<i32>> {
    let filter = rstate.filter(state).await?;
    return Ok(filter.hidden_tag_ids);
}

macro_rules! insert_csd {
    ($i:ident, $s:ident, $j:expr) => {
        let name = stringify!($s).replace("_", "-");
        let value = serde_json::to_value(&$j)?;
        $i.insert(name, value);
    };
}

pub async fn image_clientside_data<'a>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
    image: &Image,
    inner: Markup,
) -> TiberiusResult<Markup> {
    let mut data: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut client = state.get_db_client().await?;

    insert_csd!(data, aspect_ratio, image.image_aspect_ratio);
    insert_csd!(data, comment_count, image.comments_count);
    insert_csd!(data, created_at, image.created_at);
    insert_csd!(data, downvotes, image.downvotes_count);
    insert_csd!(data, faves, image.faves_count);
    insert_csd!(data, height, image.image_height.unwrap_or(0));
    insert_csd!(data, image_id, image.id);
    insert_csd!(data, image_tag_aliases, image.tags_text(&mut client).await?);
    let tag_ids: Vec<_> = image
        .get_tag_ids(&mut client)
        .await?
        .into_iter()
        .map(|x| x.tag_id)
        .collect();
    insert_csd!(data, image_tags, tag_ids);
    insert_csd!(data, score, image.score);
    // TODO: allow other than full
    insert_csd!(data, size, "full");
    insert_csd!(data, source_url, image.source_url);
    insert_csd!(data, upvotes, image.upvotes_count);
    insert_csd!(data, uris, image_thumb_urls(&image).await?);

    Ok(csd_to_markup("image-show-container", data, inner).await?)
}

pub async fn clientside_data<'a>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
) -> TiberiusResult<Markup> {
    let extra = rstate.csd_extra().await?;
    let interactions = rstate.interactions().await?;
    let user = rstate.user(state).await?;
    let filter = rstate.filter(state).await?;

    let mut data: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    insert_csd!(data, filter_id, filter.id);
    insert_csd!(data, hidden_tag_list, filter.hidden_tag_ids);
    insert_csd!(
        data,
        hidden_filter,
        filter
            .hidden_complex_str
            .as_ref()
            .map(|x| x.clone())
            .unwrap_or("".to_string())
    );
    insert_csd!(data, spoilered_tag_list, filter.spoilered_tag_ids);
    insert_csd!(
        data,
        spoilered_filter,
        filter
            .spoilered_complex_str
            .as_ref()
            .map(|x| x.clone())
            .unwrap_or("".to_string())
    );
    insert_csd!(data, user_is_signed_in, user.is_some());
    insert_csd!(data, interactions, interactions);
    if let Some(user) = user {
        insert_csd!(data, user_id, user.id);
        insert_csd!(data, user_name, user.name);
        insert_csd!(data, user_slug, user.slug);
        insert_csd!(
            data,
            user_can_edit_filter,
            if let Some(filter_user_id) = filter.user_id {
                filter_user_id == user.id
            } else {
                false
            }
        );
        insert_csd!(data, spoiler_type, user.spoiler_type);
        insert_csd!(data, watched_tag_list, user.watched_tag_ids);
        insert_csd!(data, fancy_tag_edit, user.fancy_tag_field_on_edit);
        insert_csd!(data, fancy_tag_upload, user.fancy_tag_field_on_upload);
        insert_csd!(
            data,
            ignored_tag_list,
            ignored_tag_list(state, rstate).await?
        );
        insert_csd!(
            data,
            hide_staff_tools,
            rstate.cookie_jar.get("hide_staff_tools").is_some()
        );
    } else {
        let empty_vec: Vec<i32> = Vec::new();
        insert_csd!(data, watched_tag_list, empty_vec);
        insert_csd!(data, ignored_tag_list, empty_vec);
    }

    for (k, v) in extra {
        data.insert(k.clone(), v.clone());
    }

    Ok(csd_to_markup("js-datastore", data, PreEscaped("".to_string())).await?)
}

async fn csd_to_markup<S: std::fmt::Display>(
    class: S,
    data: BTreeMap<String, serde_json::Value>,
    inner: Markup,
) -> TiberiusResult<Markup> {
    let data: Vec<String> = data
        .iter()
        .map(|(k, v)| {
            let mut s = String::new();
            let v = match v.as_str() {
                None => v.to_string(),
                Some(v) => v.to_string(),
            };
            use std::fmt::Write;
            maud::Escaper::new(&mut s)
                .write_str(&v)
                .expect("could not write data-store");
            let s = s.trim_matches('\"');
            format!("data-{}=\"{}\"", k, s)
        })
        .collect();
    let data = data.join(" ");
    let data = format!(
        r#"<div class="{}" {}>{}</div>"#,
        class,
        data,
        inner.into_string()
    );
    Ok(PreEscaped(data))
}

pub async fn container_class(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
) -> TiberiusResult<String> {
    if let Some(user) = rstate.user(state).await? {
        if user.use_centered_layout {
            return Ok("layout--center-aligned".to_string());
        }
    }
    Ok("".to_string())
}

pub async fn app(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<'_>,
    page_title: Option<PageTitle>,
    client: &mut Client,
    body: Markup,
    image: Option<Image>,
) -> TiberiusResult<Markup> {
    let meta = html! {
        meta charset="UTF-8";
        meta http-equiv="X-UA-Compatible" content="IE=edge";
        (viewport_meta_tags(rstate));
    };
    let title = html! {
        title { (
            match page_title {
                //TODO: make title customizable
                Some(title) => {
                    let title: String = title.clone().into();
                    title + " - Manebooru"
                },
                None => "Manebooru".to_string(),
            }
        ) }
    };
    let links_and_meta = html! {
        link rel="stylesheet" href=(stylesheet_path(state, rstate).await?);
        @if rstate.user(state).await?.is_some() {
            link rel="stylesheet" href=(dark_stylesheet_path(rstate)?) media="(prefers-color-scheme: dark)";
        }
        link rel="icon" href="/favicon.ico" type="image/x-icon";
        link rel="icon" href="/favicon.svg" type="image/svg+xml";
        meta name="generator" content="philomena";
        meta name="theme-color" content="#618fc3";
        meta name="format-detection" content="telephone=no";
        (csrf_meta_tag(rstate).await);
    };
    let body = html! {
        body data-theme=(rstate.theme_name(state).await?) {
            (burger());
            div.(container_class(state, rstate).await?)#container {
                (header(state.site_config(), state, rstate).await?);
                (flash_warnings(state, rstate).await?);
                main.(layout_class(rstate).await)#content { (body) }
                (footer(state, rstate).await?);
                form.hidden {
                    input.js-interaction-cache type="hidden" value="{}";
                }
                (clientside_data(state, rstate).await?);
            }
        }
    };
    let script = html! {
        script type="text/javascript" src=(static_path("js/app.js").to_string_lossy()) async="async" {}
        /*(maud::PreEscaped("</script>"));*/
        (open_graph(state, image).await?);
    };
    Ok(html! {
        (maud::DOCTYPE)
        html lang="en" {
            (meta);
            (title);
            (links_and_meta);
            (script);
            (body);
        }
    })
}
