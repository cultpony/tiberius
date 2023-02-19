use axum::headers::{HeaderMapExt, UserAgent};
use axum_extra::routing::TypedPath;
use chrono::{DateTime, NaiveDateTime, Utc};
use itertools::Itertools;
use std::{
    collections::BTreeMap,
    fmt::{write, Debug, Display},
};
use tiberius_common_html::no_avatar_svg;
use tiberius_core::{
    app::PageTitle,
    assets::{QuickTagTableContent, SiteConfig},
    error::{TiberiusError, TiberiusResult},
    request_helper::FormMethod,
    session::{Session, SessionMode},
    state::{SiteNotices, TiberiusRequestState, TiberiusState},
};
use tiberius_dependencies::axum_flash::{Flash, IncomingFlashes};

use crate::{
    api::int::oembed::PathOembed,
    pages::{
        common::routes::{cdn_host, dark_stylesheet_path, static_path, stylesheet_path},
        images::{PathSearchEmpty, PathShowImage},
        session::{PathNewSession, PathRegistration, PathSessionLogout},
        tags::PathTagsByNameShowTag,
        PathImageThumbGetSimple,
    },
};
use either::Either;
use maud::{html, Markup, PreEscaped};
use tiberius_models::{
    Badge, Channel, Client, Conversation, Filter, Forum, Image, ImageThumbType, Notification,
    SiteNotice, Tag, TagLike, User,
};
use tracing::{trace, Instrument};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CSSWidth {
    Pixels(u16),
    Ems(u16),
}

impl Display for CSSWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CSSWidth::Pixels(px) => format!("{px}px"),
            CSSWidth::Ems(em) => format!("{em}em"),
        };
        f.write_str(s.as_str())
    }
}

pub fn viewport_meta_tags<T: SessionMode>(rstate: &TiberiusRequestState<T>) -> Markup {
    let mobile_uas = ["Mobile", "webOS"];
    if let Some(value) = rstate.headers.typed_get::<UserAgent>() {
        for mobile_ua in &mobile_uas {
            if value.to_string().contains(mobile_ua) {
                return html! { meta name="viewport" content="width=device-width, initial-scale=1"; };
            }
        }
    }
    return html! { meta name="viewport" content="width=1024, initial-scale=1"; };
}

pub async fn csrf_meta_tag<T: SessionMode>(rstate: &TiberiusRequestState<T>) -> Markup {
    let csrf = rstate.csrf_token.authenticity_token();
    html! {
        meta content=(csrf) csrf-param="_csrf_token" method-param="_method" name="csrf-token";
    }
}

pub async fn csrf_input_tag<T: SessionMode>(rstate: &TiberiusRequestState<T>) -> Markup {
    let csrf = rstate.csrf_token.authenticity_token();
    html! {
        input type="hidden" name="_csrf_token" value=(csrf);
    }
}

pub fn form_method(method: FormMethod) -> Markup {
    html! {
        input type="hidden" name="_method" value=(method.to_string());
    }
}

pub fn form_submit_button(label: &str) -> Markup {
    html! {
        input type="submit" value=(label);
    }
}

#[instrument(skip(state, image))]
pub async fn open_graph(state: &TiberiusState, image: Option<Image>) -> TiberiusResult<Markup> {
    let mut client = state.get_db_client();
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
            meta property="og:url" content=(PathShowImage{ image: (image.id as u64) }.to_uri().to_string());

            @for tag in artist_tags(&image.tags(&mut client).await?) {
                meta property="dc:creator" content=(tag.full_name());
            }

            @if let Some(source_url) = &image.source_url {
                @if !source_url.is_empty() {
                    meta property="foaf:primaryTopic" content=(source_url);
                }
            }

            link rel="alternate" type="application/json-oembed" href=(PathOembed{}.to_uri().to_string()) title="oEmbed JSON Profile";
            link rel="canonical" href=(PathShowImage{ image: (image.id as u64) }.to_uri().to_string());

            @match (image.image_mime_type.as_ref().map(|x| x.as_str()), filtered) {
                (Some("video/webm"), false) => {
                    meta property="og:type" content="video.other";

                    meta property="og:image" content=(PathImageThumbGetSimple{ id: image.id as u64, thumbtype : "".to_string(), filename : image.filetypef("rendered")}.to_uri().to_string());
                    meta property="og:video" content=(PathImageThumbGetSimple{ id: image.id as u64, thumbtype : "".to_string(), filename : image.filetypef("large")}.to_uri().to_string());
                },
                (Some("image/svg+xml"), false) => {
                    meta property="og:type" content="website";
                    meta property="og:image" content=(PathImageThumbGetSimple{ id: image.id as u64, thumbtype : "".to_string(), filename : image.filetypef("rendered")}.to_uri().to_string());
                },
                (_, false) => {
                    meta property="og:type" content="website";
                    meta property="og:image" content=(PathImageThumbGetSimple{ id: image.id as u64, thumbtype : "large".to_string(), filename : image.filename()}.to_uri().to_string());
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
        nav #burger {
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
            textarea.input.input--wide.tagsinput.js-image-input.js-taginput.js-taginput-plain.hidden #image_tag_input.(ta_class) autocomplete="off" name="image.tag_input" placeholder="Add tags seperated with commas" {}
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
        button.button.button--state-success.button--separate-left.button--bold #tagsinput-save title="This button saves the tags listed above to your browser, allowing you to retrieve them again by clicking the Load button" {
            "Save"
        }
        button.button.button--state-warning.button--separate-left.button--bold #tagsinput-save title="This button loads any saved tags from your browser" {
            "Load"
        }
        button.button.button--state-danger.button--separate-left.button--bold #tagsinput-clear title="This button will clear the list of tags above" type="button" {
            "Clear"
        }
    }
}

pub fn tag_link(uri: bool, tag: &str, name: &str) -> Markup {
    //TODO: set proper title for tag description
    let uri = if uri {
        PathTagsByNameShowTag {
            tag: tag.to_string(),
        }
        .to_uri()
        .to_string()
    } else {
        "#".to_string()
    };
    html! {
        a href=(uri) data-tag-name=(tag) data-click-addtag=(tag) { (name) }
    }
}

#[instrument(skip(state))]
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

#[instrument(skip(state, rstate))]
pub async fn header<T: SessionMode>(
    site_config: &SiteConfig,
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
) -> TiberiusResult<Markup> {
    let notifications = rstate.notifications().await?;
    let mut client = state.get_db_client();
    let filter: &Filter = rstate.filter(state).await?;
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

                form.header__search.flex.flex--nowrap.flex--centered.hform action=(PathSearchEmpty{}.to_uri().to_string()) method="GET" {
                    input.input.header__input.header__input--search #q name="q" title="For terms all required, separate with ',' or 'AND'; also supports 'OR' for optional terms and '-' or 'NOT' for negation. Search with a blank query for more options or click the ? for syntax help."
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
                        form #filter-quick-form.header__filter-form action="/filters/current" method="POST" {}

                        // TODO: user change hide/spoiler form https://github.com/derpibooru/philomena/blob/355ce491accae4702f273334271813e93a261e0f/lib/philomena_web/templates/layout/_header.html.slime#L55
                        form #spoiler-quick-form.header__filter-form.hide-mobile.hide-limited-desktop action="/filters/spoiler_type" method="POST" {}


                        .dropdown.header_dropdown {
                            a.header__link.header__link-user href=(PathRegistration{}.to_uri().to_string()) {
                                //TODO: render user attribution view
                                .image-constrained."avatar--28px" {
                                    (no_avatar_svg())
                                }
                                span.header__link-user__dropdown__content.hide-mobile data-click-preventdefault="true";
                            }
                            nav.dropdown__content.dropdown__content-right.hide-mobile.js-burger-links {
                                a.header__link href=(PathRegistration{}.to_uri().to_string()) { (user.name); }
                                a.header__link href="/search?q=my:watched" { i.fa.fa-fw.fa-eye { "Watched"; } }
                                a.header__link href="/search?q=my:faves" { i.fa.fa-fw.fa-start { "Faves"; } }
                                a.header__link href="/search?q=my:upvotes" { i.fa.fa-fw.fa-arrow-up { "Upvotes"; } }
                                a.header__link href=(PathRegistration{}.to_uri().to_string()) { i.fa.fa-fw.fa-image { "Galleries"; }}
                                a.header__link href="/search?q=my:uploads" { i.fa.fa-fw.fa-upload { "Uploads"; } }
                                a.header__link href="/comments?cq=my:comments" { i.fa.fa-fw.fa-comments { "Comments"; } }
                                a.header__link href="/posts?pq=my:watched" { i.fa.fa-fw.fa-pen-square { "Posts"; } }
                                a.header__link href=(PathRegistration{}.to_uri().to_string()) { i.fa.fa-fw.fa-link { "Links"; } }
                                a.header__link href="/settings/edit" { i.fa.fa-fw.fa-cogs { "Settings"; } }
                                a.header__link href="/conversations" { i.fa.fa-fw.fa-envelope { "Messages"; } }
                                a.header__link href=(PathRegistration{}.to_uri().to_string()) { i.fa.fa-fw.fa-user { "Account"; } }
                                a.header__link href=(PathSessionLogout{}.to_uri().to_string()) { i.fa.fa-fw.fa-sign-out-alt { "Logout"; } }
                            }
                        }
                    } @else {
                        a.header__link.hide-mobile href="/filters" { (format!("Filters ({})", filter.name)) }
                        span.js-burger-links.hide-mobile {
                            a.header__link href="/settings/edit" {
                                i.fa.fa-fw.fa-cogs.hide-desktop { "Settings" }
                            }
                        }
                        a.header__link href=(PathRegistration{}.to_uri().to_string()) { "Register" }
                        a.header__link href=(PathNewSession{}.to_uri().to_string()) { "Login" }
                    }
                }
            }
        }
        nav.header.header--secondary {
            .flex.flex--centered.flex--spaced-out.flex--wrap {
                (header_navigation_links(&mut client).await?)
                @if user.as_ref().map(|x| x.role.as_str()) != Some("user") {
                    (header_staff_links())
                }
            }
        }
    })
}

#[instrument]
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
                        a.header__link href=(PathRegistration{}.to_uri().to_string())  {
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

pub fn pretty_time(date: &NaiveDateTime) -> String {
    use tiberius_dependencies::chrono_humanize::HumanTime;
    let date: DateTime<Utc> = DateTime::from_utc(date.clone(), Utc);
    let ht = HumanTime::from(date);
    format!("{}", ht)
}

#[instrument(skip(state, rstate))]
pub async fn flash_warnings<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
) -> TiberiusResult<Markup> {
    let site_notices: SiteNotices = state.site_notices().await?;
    let mut flash_msgs: Vec<PreEscaped<String>> = Vec::new();
    for (flash_lvl, flash_msg) in rstate.incoming_flashes.iter() {
        use tiberius_dependencies::axum_flash::Level;
        flash_msgs.push(match flash_lvl {
            Level::Debug => todo!(),
            Level::Info => html! { .flash.flash--success { (flash_msg) } },
            Level::Success => html! { .flash.flash--success { (flash_msg) } },
            Level::Warning => html! { .flash.flash--warning { (flash_msg) } },
            Level::Error => html! { .flash.flash--warning { (flash_msg) } },
        });
    }
    let flash_body = html! {
        @for flash_msg in flash_msgs {
            (flash_msg);
        }
    };
    let flash_pre = html! {
        @for notice in site_notices.0 {
            .flash.flash--site-notice {
                strong { (notice.title); }
                " "
                (notice.text)
                " "
                @if !notice.link.is_empty() {
                    a href=(notice.link) {
                        (notice.link_text)
                    }
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

pub async fn layout_class<T: SessionMode>(req: &TiberiusRequestState<T>) -> String {
    req.layout_class().await.to_string()
}

#[instrument(skip(state, rstate))]
pub async fn footer<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
) -> TiberiusResult<Markup> {
    let end_time = rstate.started_at;
    let time = end_time.elapsed();
    let time: f32 = time.as_secs_f32() * 1000f32; // TODO: reimplement measuring this
    let footer_data = state.footer_data();
    let site_config = state.site_config();
    let render_time = {
        #[cfg(debug_assertions)]
        let ret = format!(" (rendered in {:1.3} ms, debug)", time);
        #[cfg(not(debug_assertions))]
        let ret = format!(" (rendered in {:1.3} ms)", time);
        ret
    };
    Ok(html! {
        footer #footer {
            div #footer_content {
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
            div #serving_info {
                "Powered by "
                a href=(site_config.source_repo()) { (site_config.source_name()) }
                (render_time)
            }
        }
    })
}

pub async fn ignored_tag_list<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
) -> TiberiusResult<Vec<i32>> {
    let filter = rstate.filter(state).await?;
    return Ok(filter.hidden_tag_ids.clone());
}

macro_rules! insert_csd {
    ($i:ident, $s:ident, $j:expr) => {
        let name = stringify!($s).replace("_", "-");
        let value = serde_json::to_value(&$j)?;
        $i.insert(name, value);
    };
}

#[instrument(skip(rstate, state, inner, image), level = "debug")]
pub async fn image_clientside_data<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
    image: &Image,
    inner: PreEscaped<String>,
) -> TiberiusResult<Markup> {
    Ok(state
        .csd_cache
        .try_get_with(
            image.id as u64,
            uncached_image_clientside_data(state, rstate, image, inner),
        )
        .await
        .map_err(|e| TiberiusError::CacheError(format!("{}", e)))?)
}

#[instrument(skip(rstate, state, inner, image))]
async fn uncached_image_clientside_data<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
    image: &Image,
    inner: PreEscaped<String>,
) -> TiberiusResult<Markup> {
    let mut data: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut client = state.get_db_client();

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
    insert_csd!(data, uris, image.image_thumb_urls().await?);

    Ok(csd_to_markup("image-show-container", data, inner).await?)
}

#[instrument(skip(rstate, state))]
pub async fn clientside_data<'a, T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
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
        insert_csd!(data, spoiler_type, user.user_settings.spoiler_type);
        insert_csd!(data, watched_tag_list, user.user_settings.watched_tag_ids);
        insert_csd!(data, fancy_tag_edit, user.user_settings.fancy_tag_field_on_edit);
        insert_csd!(data, fancy_tag_upload, user.user_settings.fancy_tag_field_on_upload);
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

#[instrument(skip(class, data, inner))]
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

pub async fn container_class<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
) -> TiberiusResult<String> {
    if let Some(user) = rstate.user(state).await? {
        if user.user_settings.use_centered_layout {
            return Ok("layout--center-aligned".to_string());
        }
    }
    Ok("".to_string())
}

#[instrument]
pub async fn user_attribution<S: ToString>(
    client: &mut Client,
    user: &User,
) -> TiberiusResult<maud::Markup> {
    Ok(html! {
        strong {
            a href="user link" { (user.displayname()) }
        }
        (user_badges(user, client).await?)
    })
}

#[instrument(skip(classes))]
pub fn user_attribution_avatar<S: ToString>(
    user: &User,
    classes: S,
) -> TiberiusResult<maud::Markup> {
    let av = user.avatar();
    let classes = classes.to_string();
    let classes = if !classes.is_empty() {
        format!("image-constrained {}", classes)
    } else {
        classes
    };
    Ok(match av {
        Either::Left(url) => html! {
            div class=(classes) { img src=(url) {} }
        },
        Either::Right(markup) => html! {
            div class=(classes) { (markup) }
        },
    })
}

pub fn badge_image(
    img: Option<&String>,
    alt: String,
    title: String,
    width: u64,
    height: u64,
) -> maud::Markup {
    html! {
        img src=(img.unwrap_or(&"placeholder/url".to_string())) alt=(alt) title=(title) width=(width) height=(height) {}
    }
}

#[instrument]
pub async fn user_badges(user: &User, client: &mut Client) -> TiberiusResult<maud::Markup> {
    let badges = user.badges(client).await?;
    let (badges, overflow): (&[Badge], &[Badge]) = if badges.len() <= 10 {
        (&*badges, &[])
    } else {
        badges.split_at(10)
    };
    Ok(html! {
        .badges {
            // Grab the next 10 badges
            @for badge in badges {
                .badge {
                    (badge_image(badge.image.as_ref(), badge.title(), badge.title(), 18, 18))
                }
            }
            @if !overflow.is_empty() {
                .dropdown {
                    i.fa.fa-caret-down {}
                    .dropdown__content.block__header {
                        .badges.flex--column {
                            @for badge in badges {
                                .badge {
                                    (badge_image(badge.image.as_ref(), badge.title(), badge.title(), 18, 18))
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

//#[instrument(skip(rstate, state, client, body, image))]
pub async fn app<T: SessionMode>(
    state: &TiberiusState,
    rstate: &TiberiusRequestState<T>,
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
    let flash = rstate.flash();
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
    let body_gen_span = trace_span!("generate_body");
    let body = async {
        Ok::<_, TiberiusError>(html! {
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
        })
    }
    .instrument(body_gen_span)
    .await?;
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
