use crate::templates::common::renderer::textile_extensions;
use ammonia::{self, UrlRelative};
use maplit::hashset;
use maud::PreEscaped;
use regex::Regex;
use tiberius_dependencies::textile;

pub fn render_textile(inp: &str) -> PreEscaped<String> {
    let opts = textile::RenderOptions {
        compress: true,
        ..Default::default()
    };
    let inp = inp.replace("! ", "\\u0021");
    let inp = inp.replace("- ", "\\u002d");
    let unsafe_render = textile::render_with(inp, opts);
    let unsafe_render = textile_extensions(unsafe_render.trim());
    let safe_render = ammonia::Builder::default()
        .link_rel(Some("noopener noreferrer"))
        .url_relative(UrlRelative::PassThrough)
        .add_tag_attribute_values("span", "class", &["spoiler"])
        .strip_comments(true)
        .url_schemes(hashset!["https"])
        .clean(&unsafe_render);

    let safe_render = safe_render.to_string().replace("\\u0021", "! ");
    let safe_render = safe_render.replace("\\u002d", "- ");
    PreEscaped(safe_render)
}

/*#[cfg(test)]
mod test {
    use super::render_textile;

    #[test]
    pub fn test_common() {
        assert_eq!("<p><em>italics</em></p>", render_textile("_italics_").0);
    }

    #[test]
    pub fn test_common_bold() {
        assert_eq!("<p><strong>bold</strong></p>", render_textile("*bold*").0);
    }

    #[test]
    pub fn test_common_underline() {
        assert_eq!(
            "<p><ins>underline</ins></p>",
            render_textile("+underline+").0
        );
    }

    #[test]
    pub fn test_common_strike() {
        assert_eq!("<p>with <del>strike</del> text</p>", render_textile("with -strike- text").0);
    }

    #[test]
    pub fn test_common_super() {
        assert_eq!("<p><sup>sup</sup></p>", render_textile("^sup^").0);
    }

    #[test]
    pub fn test_common_subscript() {
        assert_eq!("<p><sub>sub</sub></p>", render_textile("~sub~").0);
    }

    #[test]
    pub fn test_common_preformat() {
        assert_eq!("<p><code>code</code></p>", render_textile("@code@").0);
    }

    #[test]
    pub fn test_common_unparsed() {
        assert_eq!(
            "<p>unparsed *not-bold* text</p>",
            render_textile("==unparsed *not-bold* text==").0
        );
    }

    #[test]
    pub fn test_common_link_onsite() {
        assert_eq!(
            "<p><a href=\"/some-link\" rel=\"noopener noreferrer\">On-site link</a></p>",
            render_textile(r#""On-site link":/some-link"#).0
        );
    }

    #[test]
    pub fn test_common_link_external() {
        assert_eq!(
            "<p><a href=\"https://external.site/\" rel=\"noopener noreferrer\">External link</a></p>",
            render_textile(r#""External link":https://external.site/"#).0
        );
    }

    #[test]
    pub fn test_common_empty() {
        assert_eq!("", render_textile("").0);
    }

    #[test]
    pub fn test_common_dot() {
        assert_eq!("<p>.</p>", render_textile(".").0);
    }

    #[test]
    pub fn test_common_image() {
        assert_eq!(
            "<p><img src=\"https://some-image\"></p>",
            render_textile("!https://some-image!").0
        );
    }

    #[test]
    pub fn test_common_imagelink() {
        assert_eq!(
            "<p><a href=\"https://some-link\" rel=\"noopener noreferrer\"><img src=\"https://some-image\"></a></p>",
            render_textile("!https://some-image!:https://some-link").0
        );
    }

    #[test]
    pub fn test_philo_specifics_spoiler() {
        assert_eq!(
            r#"<p><span class="spoiler">spoilerino</span></p>"#,
            render_textile("[spoiler]spoilerino[/spoiler]").0
        );
    }

    #[test]
    pub fn test_philo_specifics_image_embed() {
        assert_eq!(
            r#"<p><img src="/img/embed/1/"></p>"#,
            render_textile(">>1").0
        );
    }

    #[test]
    pub fn test_philo_specifics_image_embed_thumbnails() {
        assert_eq!(
            r#"<p><img src="/img/embed/1/t"></p>"#,
            render_textile(">>1t").0
        );
        assert_eq!(
            r#"<p><img src="/img/embed/1/p"></p>"#,
            render_textile(">>1p").0
        );
        assert_eq!(
            r#"<p><img src="/img/embed/1/s"></p>"#,
            render_textile(">>1s").0
        );
    }

    #[test]
    pub fn test_philo_specifics_date_strikethrough() {
        assert_eq!(
            r#"<p>2016-11-28T03:55:53</p>"#,
            render_textile("2016-11-28T03:55:53").0,
            "Do not strike through date and timestamps"
        )
    }

    #[test]
    pub fn test_philo_specifics_doublenewline() {
        assert_eq!(
            r#"<p>test<br>test</p><p>test<br>test</p>"#,
            render_textile("test\ntest\n\ntest\ntest").0.replace('\n', ""),
            "Pass new lines correctly, even if doubled"
        )
    }

    #[test]
    pub fn test_philo_dontencode_sentences() {
        assert_eq!(
            r#"<p>A! B! C.</p>"#,
            render_textile("A! B! C.").0,
            "Space seperated exclamation points should not be encoded",
        );
        assert_eq!(
            r#"<p>A – B. C-D</p>"#,
            render_textile("A - B. C-D").0,
            "Spaced minus character should not lead to <del> element encoding",
        );
    }
}
*/
