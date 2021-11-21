use ammonia::Builder;
use pulldown_cmark::{html, Options, Parser};

use crate::pages::common::renderer::markdown_extensions;

pub fn render_markdown(inp: &str) -> String {
    let inp = markdown_extensions(inp);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(&inp, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
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

    #[test]
    pub fn test_common() {
        assert_eq!("<p><em>italics</em></p>", render_markdown("*italics*"));
        assert_eq!("<p><strong>bold</strong></p>", render_markdown("**bold**"));
        assert_eq!("<p><del>strike</del></p>", render_markdown("~~strike~~"));
        assert_eq!("<p><code>code</code></p>", render_markdown("`code`"));
        assert_eq!(
            "<p><a href=\"/some-link\">On-site link</a></p>",
            render_markdown(r#"[On-site link](/some-link)"#)
        );
        assert_eq!(
            "<p><a href=\"https://external.site/\">External link</a></p>",
            render_markdown(r#"[External link](https://external.site/)"#)
        );
        assert_eq!("", render_markdown(""));
        assert_eq!("<p>.</p>", render_markdown("."));
        assert_eq!(
            "<p><img src=\"http://some-image!\" alt=\"\"></p>",
            render_markdown("![](http://some-image!)")
        );
        assert_eq!(
            "<p><a href=\"http://some-link\"><img src=\"http://some-image\" alt=\"\"></a></p>",
            render_markdown("[![](http://some-image)](http://some-link)")
        );
    }

    #[test]
    pub fn test_philo_specifics_spoiler() {
        assert_eq!(
            r#"<p><span class="spoiler">spoilerino</span></p>"#,
            render_markdown("[spoiler]spoilerino[/spoiler]")
        );
    }

    #[test]
    pub fn test_philo_specifics_image_embed() {
        assert_eq!(
            r#"<p><img src="/img/embed/1/" alt=""></p>"#,
            render_markdown(">>1")
        );
    }

    #[test]
    pub fn test_philo_specifics_image_embed_thumbnails() {
        assert_eq!(
            r#"<p><img src="/img/embed/1/t" alt=""></p>"#,
            render_markdown(">>1t")
        );
        assert_eq!(
            r#"<p><img src="/img/embed/1/p" alt=""></p>"#,
            render_markdown(">>1p")
        );
        assert_eq!(
            r#"<p><img src="/img/embed/1/s" alt=""></p>"#,
            render_markdown(">>1s")
        );
    }
}
