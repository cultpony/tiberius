use crate::session::SessionMode;


pub fn render_link<S: AsRef<str>>(link: impl axum_extra::routing::TypedPath, text: S) -> maud::PreEscaped<String> {
    let url = link.to_uri().to_string();
    let texts: &str = text.as_ref();
    maud::html!{
        a href=(url) { (texts) }
    }
}

pub fn render_ext_link<S: AsRef<str>>(link: axum::http::Uri, text: S) -> maud::PreEscaped<String> {
    assert!(link.host().is_some(), "External Links must have host");
    let url = link.to_string();
    let texts: &str = text.as_ref();
    maud::html!{
        a href=(url) rel="external nofollow noopener noreferrer" referrerpolicy="no-referrer" { (texts) }
    }
}

#[cfg(test)]
mod test {
    use crate::links::{render_link, render_ext_link};
    use crate::session::Testing;

    #[test]
    pub fn test_link_basic_generation() {
        #[derive(axum_extra::routing::TypedPath, Debug, serde::Deserialize)]
        #[typed_path("/example/url")]
        struct BasicUrl {}
        assert_eq!(maud::html!{
            a href="/example/url" { "test" }
        }.0, render_link(BasicUrl{}, "test").0);
    }

    #[test]
    pub fn test_ext_link_generation() {
        use std::str::FromStr;
        assert_eq!(maud::html!{
            a href="https://example.com/folder/file" rel="external nofollow noopener noreferrer" referrerpolicy="no-referrer" { "test" }
        }.0, render_ext_link(axum::http::Uri::from_str("https://example.com/folder/file").unwrap(), "test").0);
    }

    #[test]
    #[should_panic = "External Links must have host"]
    pub fn test_ext_link_generation_non_external() {
        use std::str::FromStr;
        assert_eq!("MUST PANIC", render_ext_link(axum::http::Uri::from_str("/folder/file").unwrap(), "test").0);
    }
}