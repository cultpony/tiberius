use crate::pages::common::renderer::textile_extensions;

pub fn render_textile(inp: &str) -> String {
    let mut opts = textile::RenderOptions::default();
    opts.compress = true;
    textile_extensions(&textile::render_with(inp.to_string(), opts))
}

#[cfg(test)]
mod test {
    use super::render_textile;

    #[test]
    pub fn test_common() {
        assert_eq!("<p><em>italics</em></p>", render_textile("_italics_"));
        assert_eq!("<p><strong>bold</strong></p>", render_textile("*bold*"));
        assert_eq!("<p><ins>underline</ins></p>", render_textile("+underline+"));
        assert_eq!("<p><del>strike</del></p>", render_textile("-strike-"));
        assert_eq!("<p><sup>sup</sup></p>", render_textile("^sup^"));
        assert_eq!("<p><sub>sub</sub></p>", render_textile("~sub~"));
        assert_eq!("<p><code>code</code></p>", render_textile("@code@"));
        assert_eq!(
            "<p>unparsed *not-bold* text</p>",
            render_textile("==unparsed *not-bold* text==")
        );
        assert_eq!(
            "<p><a href=\"/some-link\">On-site link</a></p>",
            render_textile(r#""On-site link":/some-link"#)
        );
        assert_eq!(
            "<p><a href=\"https://external.site/\">External link</a></p>",
            render_textile(r#""External link":https://external.site/"#)
        );
        assert_eq!("", render_textile(""));
        assert_eq!("<p>.</p>", render_textile("."));
        assert_eq!(
            "<p><img src=\"http://some-image\"></p>",
            render_textile("!http://some-image!")
        );
        assert_eq!(
            "<p><a href=\"http://some-link\"><img src=\"http://some-image\"></a></p>",
            render_textile("!http://some-image!:http://some-link")
        );
    }

    #[test]
    pub fn test_philo_specifics_spoiler() {
        assert_eq!(r#"<p><span class="spoiler">spoilerino</span></p>"#, render_textile("[spoiler]spoilerino[/spoiler]"));
    }

    #[test]
    pub fn test_philo_specifics_image_embed() {
        assert_eq!(r#"<p><img src="/img/embed/1/"></img></p>"#, render_textile(">>1"));
    }

    #[test]
    pub fn test_philo_specifics_image_embed_thumbnails() {
        assert_eq!(r#"<p><img src="/img/embed/1/t"></img></p>"#, render_textile(">>1t"));
        assert_eq!(r#"<p><img src="/img/embed/1/p"></img></p>"#, render_textile(">>1p"));
        assert_eq!(r#"<p><img src="/img/embed/1/s"></img></p>"#, render_textile(">>1s"));
    }
}
