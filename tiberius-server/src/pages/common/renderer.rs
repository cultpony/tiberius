use regex::Regex;

pub mod textile;
pub mod markdown;

pub(crate) fn textile_extensions(inp: &str) -> String {
    lazy_static::lazy_static! {
        static ref TEXTILE_IMAGE_SYNTAX: Regex = Regex::new(r#">>(?P<image>\d+)(?P<flag>\w?)"#).expect("core regex failure");
        static ref TEXTILE_SPOILER_SYNTAX: Regex = Regex::new(r#"\[spoiler\](?P<spoilered>[^\[]*)(\[/spoiler\])"#).expect("core regex failure");
    }
    let inp = TEXTILE_IMAGE_SYNTAX.replace_all(inp, r#"<img src="/img/embed/$image/$flag"></img>"#);
    let inp = TEXTILE_SPOILER_SYNTAX.replace_all(&inp, r#"<span class="spoiler">$spoilered</span>"#);
    inp.to_string()
}