use regex::Regex;

pub mod markdown;
pub mod textile;

pub(crate) fn textile_extensions(inp: &str) -> String {
    lazy_static::lazy_static! {
        static ref TEXTILE_IMAGE_SYNTAX: Regex = Regex::new(r#">>(?P<image>\d+)(?P<flag>\w?)"#).expect("core regex failure");
        static ref TEXTILE_SPOILER_SYNTAX: Regex = Regex::new(r#"\[spoiler\](?P<spoilered>[^\[]*)\[/spoiler\]"#).expect("core regex failure");
        static ref TEXTILE_BLOCKQUOTE_SYNTAX: Regex = Regex::new(r#"\[bq\](?P<bq>[^\[]*)(\[/bq\])"#).expect("core regex failure");
        static ref TEXTILE_UNDO_DATE_STRIKES_SYNTAX: Regex = Regex::new(r#"(?P<dateyr>\d)<del>(?P<datemon>[^<]*)</del>(?P<datedy>\d)"#).expect("core regex failure");
    }
    let inp =
        TEXTILE_IMAGE_SYNTAX.replace_all(&inp, r#"<img src="/img/embed/$image/$flag"></img>"#);
    let inp =
        TEXTILE_SPOILER_SYNTAX.replace_all(&inp, r#"<span class="spoiler">$spoilered</span>"#);
    let inp = TEXTILE_BLOCKQUOTE_SYNTAX.replace_all(&inp, "<blockquote>$bq</blockquote>");
    let inp = TEXTILE_UNDO_DATE_STRIKES_SYNTAX.replace_all(&inp, "$dateyr-$datemon-$datedy");
    // handle double newlines properly
    let inp = inp.replace("</p><p>", "<br><br>");
    inp.to_string()
}

pub(crate) fn markdown_extensions(inp: &str) -> String {
    lazy_static::lazy_static! {
        static ref TEXTILE_IMAGE_SYNTAX: Regex = Regex::new(r#">>(?P<image>\d+)(?P<flag>\w?)"#).expect("core regex failure");
        static ref TEXTILE_SPOILER_SYNTAX: Regex = Regex::new(r#"\[spoiler\](?P<spoilered>[^\[]*)(\[/spoiler\])"#).expect("core regex failure");
        static ref TEXTILE_NEWSPOILER_SYNTAX: Regex = Regex::new(r#"(\|\|)(?P<spoilered>[^|]+)(\|\|)"#).expect("core regex failure");
    }
    let inp = TEXTILE_IMAGE_SYNTAX.replace_all(inp, r#"![](/img/embed/$image/$flag)"#);
    let inp =
        TEXTILE_SPOILER_SYNTAX.replace_all(&inp, r#"<span class="spoiler">$spoilered</span>"#);
    let inp =
        TEXTILE_NEWSPOILER_SYNTAX.replace_all(&inp, r#"<span class="spoiler">$spoilered</span>"#);
    inp.to_string()
}
