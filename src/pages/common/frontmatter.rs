use std::{
    collections::BTreeMap,
    fmt::{write, Debug},
};

use crate::{
    app::{common::Query, HTTPReq, PageTitle},
    assets::AssetLoaderRequestExt,
    pages::common::{
        flash::{get_flash, Flash},
        routes::{
            api_json_oembed_url, cdn_host, dark_stylesheet_path, forum_route,
            gallery_patch_current_user, image_url, login_path, logout_path, path2url,
            profile_artist_path_current_user, profile_path_current_user, registration_path,
            static_path, stylesheet_path, thumb_url,
        },
    },
    request_helper::CSRFToken,
    request_timer::RequestTimerRequestExt,
    session::{SessionExt, SessionReqExt},
};
use anyhow::Result;
use either::Either;
use log::trace;
use maud::{html, Markup, PreEscaped};
use philomena_models::{
    Channel, Client, Conversation, Filter, Forum, Image, ImageThumbType, Notification, SiteNotice,
    Tag, User,
};

pub fn viewport_meta_tags(req: &HTTPReq) -> Markup {
    let mobile_uas = ["Mobile", "webOS"];
    if let Some(value) = req.header(tide::http::headers::USER_AGENT) {
        for mobile_ua in &mobile_uas {
            if value.to_string().contains(mobile_ua) {
                return html! { meta name="viewport" content="width=device-width, initial-scale=1"; };
            }
        }
    }
    return html! { meta name="viewport" content="width=1024, initial-scale=1"; };
}

pub fn csrf_meta_tag(req: &HTTPReq) -> Markup {
    let session = req.session();
    let csrf = session.get::<CSRFToken>("csrf_token");
    let csrf: Option<String> = csrf.map(|x| x.into());
    match csrf {
        None => html! {},
        Some(csrf) => html! {
            meta content=(csrf) csrf-param="_csrf_token" method-param="_method" name="csrf-token";
        },
    }
}

pub fn theme_name(req: &HTTPReq) -> &str {
    if let Some(user) = req.ext::<User>() {
        user.theme.as_str()
    } else {
        "default"
    }
}

pub async fn open_graph(req: &HTTPReq, client: &mut Client) -> Result<Markup> {
    let image = req.ext::<Image>();
    let filtered = !image.map(|x| x.thumbnails_generated).unwrap_or(false);
    let description = image
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
            meta property="og:url" content=(image_url(Either::Right(image)).to_string_lossy().to_string());

            @for tag in artist_tags(&image.tags(client).await?) {
                meta property="dc:creator" content=(tag.full_name());
            }

            @if let Some(source_url) = &image.source_url {
                @if !source_url.is_empty() {
                    meta property="foaf:primaryTopic" content=(source_url);
                }
            }

            link rel="alternate" type="application/json-+oembed" href=(api_json_oembed_url(req)?) title="oEmbed JSON Profile";
            link rel="canonical" href=(path2url(req, image_url(either::Right(image)))?);

            @match (image.image_mime_type.as_ref().map(|x| x.as_str()), filtered) {
                (Some("video/webm"), false) => {
                    meta property="og:type" content="video.other";
                    meta property="og:image" content=(path2url(req, thumb_url(req, client, either::Right(image), ImageThumbType::Rendered).await?)?);
                    meta property="og:video" content=(path2url(req, thumb_url(req, client, either::Right(image), ImageThumbType::Large).await?)?);
                },
                (Some("image/svg+xml"), false) => {
                    meta property="og:type" content="website";
                    meta property="og:image" content=(path2url(req, thumb_url(req, client, either::Right(image), ImageThumbType::Rendered).await?)?);
                },
                (_, false) => {
                    meta property="og:type" content="website";
                    meta property="og:image" content=(path2url(req, thumb_url(req, client, either::Right(image), ImageThumbType::Large).await?)?);
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
            a href="/search?q=first_seen_at.gt:3 days ago&amp;sf=wilson_score&amp;sd=desc" { i.fas.fa-fw.fa-poll {} "Rankings" }
            a href="/filters" { i.fa.fa-fw.fa-filter {} "Filters" }
            a href="/galleries" { i.fa.fa-fw.fa-image {} "Galleries" }
            a href="/comments" { i.fa.fa-fw.fa-comments {} "Comments" }
            a href="/commissions" { i.fa.fa-fw.fa-address-card {} "Commissions" }
            a href="/channels" { i.fa-fw.fa-podcasts {} "Channels" }
            a href="/pages/donations" { i.fa.fa-fw.fa-heart {} "Donate" }
        }
    }
}

pub async fn header(req: &HTTPReq, client: &mut Client) -> Result<Markup> {
    trace!("preloading data for header html");
    let user = req.ext::<User>();
    let notifications = req.ext::<Vec<Notification>>();
    let conversations = req.ext::<Vec<Conversation>>();
    let filter = req.ext::<Filter>();
    let current_filter = filter
        .map(|x| x.name().to_string())
        .unwrap_or(Filter::default_filter(client).await?.name().to_string());
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
                        span.fa__text.hide-limited-desktop.hide_mobile { (req.site_config().site_name()) }
                    }
                    a.header__link.hide_mobile href="/images/new" title="Upload" {
                        i.fa.fa-upload {}
                    }
                    form.header__search.flex.flex--nowrap.flex--centered.hform {
                        input.input.header__input.header__input--search#q name="q" title="For terms all required, separate with ',' or 'AND'; also supports 'OR' for optional terms and '-' or 'NOT' for negation. Search with a blank query for more options or click the ? for syntax help."
                            value=(req.ext::<Query>().unwrap_or(&Query::empty()).to_string()) placeholder="Search" autocapitalize="none";
                    }

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

                    .flex.flex--centered.flex--no-wrap.header__force-right {
                        @if let Some(user) = user {
                            a.header__link href="/notifications" title="Notification" {
                                i.fa-embedded-notification {
                                    span.js-notification-ticker.fa__text.header__counter data-notification-count=(notifications.map(|x| x.len()).unwrap_or_default());
                                }
                            }

                            @if let Some(conversations) = conversations {
                                a.header__link href="/conversations" title="Conversations" {
                                    @if conversations.len() > 0 {
                                        i.fa-embedded-unread-message {
                                            span.fa-embedded__text.header__counter {
                                                (conversations.len());
                                            }
                                        }
                                    } else {
                                        i.fa-embedded-message {
                                            span.fa-embedded__text.header__counter {
                                                "0";
                                            }
                                        }
                                    }
                                }
                            }

                            a.header_link.hide-mobile href="/filters" title="Filters" {
                                i.fa.fa-filter {}
                                span.hide-limited-desktop { "Filters"; }
                            }

                            // TODO: user change filter form https://github.com/derpibooru/philomena/blob/355ce491accae4702f273334271813e93a261e0f/lib/philomena_web/templates/layout/_header.html.slime#L52

                            // TODO: user change hide/spoiler form https://github.com/derpibooru/philomena/blob/355ce491accae4702f273334271813e93a261e0f/lib/philomena_web/templates/layout/_header.html.slime#L55

                            .dropdown.header_dropdown {
                                a.header__link.header__link.user href=(path2url(req, profile_path_current_user(req))?) {
                                    //TODO: render user attribution view
                                    span.header__link-user__dropdown__content.hide-mobile.js-burger-links data-click-preventdefault="true";
                                }
                                nav.dropdown__content.dropdown_content-right.hdie-mobile.js-burger-links {
                                    a.header__link href=(path2url(req, profile_path_current_user(req))?) { (user.name); }
                                    a.header__link href="/search?q=my:watched" { i.fa.fa-fw.fa-eye { "Watched"; } }
                                    a.header__link href="/search?q=my:faves" { i.fa.fa-fw.fa-start { "Faves"; } }
                                    a.header__link href="/search?q=my:upvotes" { i.fa.fa-fw.fa-arrow-up { "Upvotes"; } }
                                    a.header__link href=(path2url(req, gallery_patch_current_user(req))?) { i.fa.fa-fw.fa-image { "Galleries"; }}
                                    a.header__link href="/search?q=my:uploads" { i.fa.fa-fw.fa-upload { "Uploads"; } }
                                    a.header__link href="/comments?cq=my:comments" { i.fa.fa-fw.fa-comments { "Comments"; } }
                                    a.header__link href="/posts?pq=my:watched" { i.fa.fa-fw.fa-pen-square { "Posts"; } }
                                    a.header__link href=(path2url(req, profile_artist_path_current_user(req))?) { i.fa.fa-fw.fa-link { "Links"; } }
                                    a.header__link href="/settings/edit" { i.fa.fa-fw.fa-cogs { "Settings"; } }
                                    a.header__link href="/conversations" { i.fa.fa-fw.fa-envelope { "Messages"; } }
                                    a.header__link href=(path2url(req, registration_path())?) { i.fa.fa-fw.fa-user { "Account"; } }
                                    a.header__link href=(path2url(req, logout_path())?) { i.fa.fa-fw.fa-sign-out-alt { "Logout"; } }
                                }
                            }
                        } @else {
                            a.header__link.hide-mobile href="/filters" { (format!("Filters ({})", current_filter)) }
                            span.js-burger-links.hide-mobile {
                                a.header__link href="/settings/edit" {
                                    i.fa.fa-fw.fa-cogs.hide-desktop { "Settings" }
                                }
                            }
                            a.header__link href=(path2url(req, registration_path())?) { "Register" }
                            a.header__link href=(path2url(req, login_path())?) { "Login" }
                        }
                    }
                }
            }
        }
        nav.header.header--secondary {
            .flex.flex--centered.flex--spaced-out.flex--wrap {
                (header_navigation_links(req, client).await?)
                @if user.map(|x| x.role.as_str()) != Some("user") {
                    (header_staff_links(req))
                }
            }
        }
    })
}

pub async fn header_navigation_links<'a>(req: &HTTPReq, client: &mut Client) -> Result<Markup> {
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
                        a.header__link href=(path2url(req, forum_route(&forum)?)?) {
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

pub fn header_staff_links(req: &HTTPReq) -> Markup {
    html! {
        .flex.flex--cenetered.header--secondary__admin-links.stretched-mobile-links.js-staff-action {
            //TODO: add staff links
        }
    }
}

pub fn flash_warnings(req: &mut HTTPReq) -> Result<Markup> {
    let site_notices: Option<Vec<SiteNotice>> = req.ext::<Vec<SiteNotice>>().cloned();
    let site_notices = site_notices.unwrap_or_default();
    Ok(html! {
        @for notice in site_notices {
            .flash.flash--site-notice {
                strong { (notice.title); }
                (notice.text)
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
        noscript.flash.flash--warning {
            strong { "You don't appear to have Javascript enabled"; }
            "If you're using an add-on like NoScript, please allow ";
            (cdn_host(req));
            " for the site to work properly.";
        }
        @for flash in get_flash(req)? {
            @match flash {
                Flash::Info(text) => { .flash.flash--success { (text) } }
                Flash::Alert(text) => { .flash.flash--warning { (text) } }
                Flash::Error(text) => { .flash.flash--warning { (text) } }
                Flash::Warning(text) => { .flash.flash--warning { (text) } }
                Flash::None => {},
            }
        }
    })
}

pub struct LayoutClass(String);

pub fn layout_class(req: &HTTPReq) -> &str {
    if let Some(layout_class) = req.ext::<LayoutClass>() {
        return layout_class.0.as_str();
    }
    "layout--narrow"
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct FooterData {
    pub cols: Vec<String>,
    #[serde(flatten)]
    pub rows: BTreeMap<String, Vec<FooterRow>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct FooterRow {
    pub title: String,
    #[serde(with = "either::serde_untagged")]
    pub url: Either<url::Url, std::path::PathBuf>,
    #[serde(default)]
    pub bold: bool,
}

impl FooterRow {
    pub fn url(&self, req: &HTTPReq) -> Result<String> {
        match &self.url {
            Either::Left(url) => Ok(url.to_string()),
            Either::Right(path) => Ok(path2url(req, path)?.to_string()),
        }
    }
}

pub fn footer(req: &HTTPReq) -> Result<Markup> {
    let time = req.expired_time_ms();
    let footer_data = req.footer_data();
    Ok(html! {
        footer#footer {
            div#footer_content {
                @for column in &footer_data.cols {
                    .footercol {
                        h5 { (column) }
                        @for row in &footer_data.rows[column] {
                            @if row.bold {
                                strong { a href=(row.url(req)?) { (row.title) } }
                            } @else {
                                a href=(row.url(req)?) { (row.title) }
                            }
                            br;
                        }
                    }
                }
            }
            div#serving_info {
                "Powered by "
                a href=(req.site_config().source_repo()) { (req.site_config().source_name()) }
                (format!(" (rendered in {:1.3} ms)", time))
            }
        }
    })
}

pub fn ignored_tag_list<'a>(req: &'a HTTPReq) -> &'a [i32] {
    let session = req.session();
    if let Some(filter) = session.active_filter(req) {
        return filter.hidden_tag_ids.as_slice();
    }
    &[]
}

pub type ClientSideExtra = std::collections::BTreeMap<String, serde_json::Value>;
pub type Interactions = Vec<()>;

pub fn clientside_data<'a>(req: &'a HTTPReq) -> Result<Markup> {
    let extra = req.ext::<ClientSideExtra>();
    let interactions = req.ext::<Interactions>();
    let user = req.ext::<User>();
    let filter = req.ext::<Filter>();

    macro_rules! insert_csd {
        ($i:ident, $s:ident, $j:expr) => {
            let name = stringify!($s).replace("_", "-");
            $i.insert(name, serde_json::to_value(&$j)?);
        };
    }

    let mut data: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    if let Some(filter) = filter {
        insert_csd!(data, filter_id, filter.id);
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
    }
    insert_csd!(data, user_is_signed_in, user.is_some());
    insert_csd!(data, interactions, interactions.unwrap_or(&Vec::new()));
    if let Some(user) = user {
        insert_csd!(data, user_id, user.id);
        insert_csd!(data, user_name, user.name);
        insert_csd!(data, user_slug, user.slug);
        insert_csd!(
            data,
            user_can_edit_filter,
            if let Some(filter) = filter {
                if let Some(filter_user_id) = filter.user_id {
                    filter_user_id == user.id
                } else {
                    false
                }
            } else {
                false
            }
        );
        insert_csd!(data, spoiler_type, user.spoiler_type);
        insert_csd!(data, watched_tag_list, user.watched_tag_ids);
        insert_csd!(data, fancy_tag_edit, user.fancy_tag_field_on_edit);
        insert_csd!(data, fancy_tag_upload, user.fancy_tag_field_on_upload);
        insert_csd!(data, ignored_tag_list, ignored_tag_list(req));
        insert_csd!(
            data,
            hide_staff_tools,
            req.cookie("hide_staff_tools").is_some()
        );
    }

    if let Some(extra) = extra {
        for (k, v) in extra {
            data.insert(k.clone(), v.clone());
        }
    }
    let data: Vec<String> = data
        .iter()
        .map(|(k, v)| {
            let mut s = String::new();
            use std::fmt::Write;
            maud::Escaper::new(&mut s)
                .write_str(&v.to_string())
                .expect("could not write data-store");
            let s = s.trim_matches('\"');
            format!("data-{}=\"{}\"", k, s)
        })
        .collect();
    let data = data.join(" ");
    let data = format!(r#"<div class="js-datastore" {}></div>"#, data);
    Ok(PreEscaped(data))
}

pub fn container_class(req: &HTTPReq) -> String {
    if let Some(user) = req.ext::<User>() {
        if user.use_centered_layout {
            return "layout--center-aligned".to_string();
        }
    }
    "".to_string()
}

pub async fn app(req: &mut HTTPReq, mut client: Client, body: Markup) -> Result<Markup> {
    Ok(html! {
        (maud::DOCTYPE)
        html lang="en" {
            meta charset="UTF-8";
            meta http-equiv="X-UA-Compatible" content="IE=edge";
            (viewport_meta_tags(&req));

            title { (
                match req.ext::<PageTitle>() {
                    //TODO: make title customizable
                    Some(title) => {
                        let title: String = title.clone().into();
                        title + " - Manebooru"
                    },
                    None => "Manebooru".to_string(),
                }
            ) }
            link rel="stylesheet" href=(stylesheet_path(&req)?);
            @if req.user().is_some() {
                link rel="stylesheet" href=(dark_stylesheet_path(&req)?) media="(prefers-color-scheme: dark)";
            }
            link rel="icon" href="/favicon.ico" type="image/x-icon";
            link rel="icon" href="/favicon.svg" type="image/svg+xml";
            meta name="generator" content="philomena";
            meta name="theme-color" content="#618fc3";
            meta name="format-detection" content="telephone=no";
            (csrf_meta_tag(&req));
            script type="text/javascript" src=(static_path(&req, "js/app.js").to_string_lossy()) async="async" {}
            (maud::PreEscaped("</script>"));
            (open_graph(&req, &mut client).await?);
            body data-theme=(theme_name(&req)) {
                (burger());
                div.(container_class(&req))#container {
                    (header(&req, &mut client).await?);
                    (flash_warnings(req)?);
                    main.(layout_class(&req))#content { (body) }
                    (footer(&req)?);
                    form.hidden {
                        input.js-interaction-cache type="hidden" value="{}";
                    }
                    (clientside_data(&req)?);
                }
            }
        }
    })
}
