use ammonia::Builder;
//use pulldown_cmark::{html, Options, Parser};
use tiberius_dependencies::comrak;
use comrak::{markdown_to_html, ComrakOptions};
use std::collections::HashMap;

use crate::pages::common::renderer::markdown_extensions;

fn common_options(meta: &Meta) -> ComrakOptions {
    let mut options = ComrakOptions::default();
    options.extension.autolink = true;
    options.extension.table = true;
    options.extension.description_lists = true;
    options.extension.superscript = true;
    options.extension.strikethrough = true;
    options.extension.philomena = true;
    options.parse.smart = true;
    options.render.hardbreaks = true;
    options.render.github_pre_lang = true;
    // TODO: camoify images
    options.extension.camoifier = None;

    //options.extension.camoifier = Some(|s| camo::image_url(s).unwrap_or_else(|| String::from("")));

    options.extension.philomena_domains = if meta.sites.is_empty() { None } else { Some(meta.sites.clone()) };
    options.extension.philomena_replacements = Some(meta.idmap.clone());

    /*if let Ok(domains) = env::var("SITE_DOMAINS") {
        options.extension.philomena_domains = Some(domains.split(',').map(|s| s.to_string()).collect::<Vec<String>>());
    }*/

    options
}

#[derive(Default, Clone)]
pub struct Meta {
    pub idmap: HashMap<String, String>,
    pub sites: Vec<String>,
}

pub fn render_markdown(inp: &str, meta: Option<&Meta>) -> String {
    let meta = meta.cloned().unwrap_or_default();
    //let inp = markdown_extensions(inp);
    let options = common_options(&meta);
    let html_output = markdown_to_html(inp, &options);
    Builder::default()
        .add_allowed_classes("span", &["spoiler"])
        .add_url_schemes(&["img"])
        .link_rel(None)
        .clean(&*html_output.trim().to_string())
        .to_string()
}

#[cfg(test)]
mod test {
    use super::render_markdown;
    use super::Meta;
    use std::collections::HashMap;

    #[test]
    #[ignore = "only verifying runtime"]
    pub fn test_longfile() {
        let file = include_str!("./markdown_test_hard.md");
        let start = std::time::Instant::now();
        for _ in 0..50 {
            let render = render_markdown(file, None);
        }
        let elapsed = start.elapsed();
        assert!(false, "Took {elapsed:?}");
    }

    #[test]
    pub fn test_common() {
        assert_eq!("<div><em>italics</em></div>", render_markdown("*italics*", None));
        assert_eq!("<div><strong>bold</strong></div>", render_markdown("**bold**", None));
        assert_eq!("<div><del>strike</del></div>", render_markdown("~~strike~~", None));
        assert_eq!("<div><code>code</code></div>", render_markdown("`code`", None));
        assert_eq!(
            "<div><a href=\"/some-link\">On-site link</a></div>",
            render_markdown(r#"[On-site link](/some-link)"#, None)
        );
        assert_eq!(
            "<div><a href=\"https://external.site/\">External link</a></div>",
            render_markdown(r#"[External link](https://external.site/)"#, None)
        );
        assert_eq!("", render_markdown("", None));
        assert_eq!("<div>.</div>", render_markdown(".", None));
        assert_eq!(
            "<div><span class=\"\"><img src=\"http://some-image!\" alt=\"\"></span></div>",
            render_markdown("![](http://some-image!)", None)
        );
        assert_eq!(
            "<div><a href=\"http://some-link\"><span class=\"\"><img src=\"http://some-image\" alt=\"\"></span></a></div>",
            render_markdown("[![](http://some-image)](http://some-link)", None)
        );
    }

    #[test]
    pub fn test_philo_specifics_spoiler() {
        assert_eq!(
            r#"<div><span class="spoiler">spoilerino</span></div>"#,
            render_markdown("||spoilerino||", None)
        );
        /*assert_eq!(
            r#"<p><span class="spoiler"> spoilerino </span></p>"#,
            render_markdown("|| spoilerino ||", None)
        );*/
    }

    #[test]
    pub fn test_philo_specifics_image_embed() {
        assert_eq!(
            r#"<div><img src="/img/embed/1/" alt=""></div>"#,
            render_markdown(">>1", Some(&Meta{
                idmap: {
                    let mut hm = HashMap::new();
                    hm.insert("1".to_string(), "<img src=\"/img/embed/1/\" alt=\"\">".to_string());
                    hm
                },
                sites: Vec::new(),
            }))
        );
    }

    #[test]
    pub fn test_philo_specifics_image_embed_thumbnails() {
        assert_eq!(
            r#"<div><img src="/img/embed/1/t" alt=""></div>"#,
            render_markdown(">>1t", Some(&Meta{
                idmap: {
                    let mut hm = HashMap::new();
                    hm.insert("1t".to_string(), "<img src=\"/img/embed/1/t\" alt=\"\">".to_string());
                    hm
                },
                sites: Vec::new(),
            }))
        );
        assert_eq!(
            r#"<div><img src="/img/embed/1/p" alt=""></div>"#,
            render_markdown(">>1p", Some(&Meta{
                idmap: {
                    let mut hm = HashMap::new();
                    hm.insert("1p".to_string(), "<img src=\"/img/embed/1/p\" alt=\"\">".to_string());
                    hm
                },
                sites: Vec::new(),
            }))
        );
        assert_eq!(
            r#"<div><img src="/img/embed/1/s" alt=""></div>"#,
            render_markdown(">>1s", Some(&Meta{
                idmap: {
                    let mut hm = HashMap::new();
                    hm.insert("1s".to_string(), "<img src=\"/img/embed/1/s\" alt=\"\">".to_string());
                    hm
                },
                sites: Vec::new(),
            }))
        );
    }

    #[test]
    pub fn test_philo_unnest() {
        assert_eq!(
            concat!(
                "<blockquote>\n",
                  "<div>a</div>\n",
                  "<blockquote>\n",
                    "<div>b</div>\n",
                    "<blockquote>\n",
                      "<div>c</div>\n",
                    "</blockquote>\n",
                  "</blockquote>\n",
                  "<div>d</div>\n",
                "</blockquote>"
            ),
            render_markdown(concat!(
                "> a\n",
                "> > b\n",
                "> > > c\n",
                "> d\n"
            ), None)
        )
    }
}
