use regex::Regex;

pub mod textile;
pub mod markdown;

pub(crate) fn textile_extensions(inp: &str) -> String {
    lazy_static::lazy_static! {
        static ref TEXTILE_IMAGE_SYNTAX: Regex = Regex::new(r#"(>>(?P<image>\d+)(?P<flag>\w?))"#).expect("core regex failure");
    }
    let caps = TEXTILE_IMAGE_SYNTAX.captures(inp);
    if let Some(caps) = caps {
        let image = caps.name("image");
        let flag = caps.name("flag");
    }
    inp.to_string()
}