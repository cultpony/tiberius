use tiberius_dependencies::{lazy_static, regex::Regex};

lazy_static::lazy_static! {
    static ref DESTRUCTIVE_SLUG_NONPRINTABLE: Regex = Regex::new(r#"[^ -~]"#).unwrap();
    static ref DESTRUCTIVE_SLUG_NONALPHARUNS: Regex = Regex::new(r#"[^a-zA-Z0-9]+"#).unwrap();
    static ref DESTRUCTIVE_SLUG_STARTENDHYPHENS: Regex = Regex::new(r#"^-|-$"#).unwrap();
}

pub fn sluggify<S: AsRef<str>>(data: S) -> String {
    let data: &str = data.as_ref();
    let data: String = data.to_string();
    let data = data.replace('-', "-dash-");
    let data = data.replace('/', "-fwslash-");
    let data = data.replace('\\', "-bwslash-");
    let data = data.replace(':', "-colon-");
    let data = data.replace('.', "-dot-");
    let data = data.replace('+', "-plus-");
    let data = data.replace(' ', "+");

    data.to_ascii_lowercase()
}

pub fn destructive_sluggify<S: AsRef<str>>(data: S) -> String {
    let data: &str = data.as_ref();
    let data: String = data.to_string();
    let data = DESTRUCTIVE_SLUG_NONPRINTABLE.replace_all(&data, "");
    let data = DESTRUCTIVE_SLUG_NONALPHARUNS.replace_all(&data, "-");
    let data = DESTRUCTIVE_SLUG_STARTENDHYPHENS.replace_all(&data, "");
    data.to_ascii_lowercase()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_desc_slugs() {
        assert_eq!(
            "time-wasting-thread-3-0-sfw-no-explicit-grimdark",
            destructive_sluggify("Time-Wasting Thread 3.0 (SFW - No Explicit/Grimdark)")
        );
        assert_eq!("", destructive_sluggify("~`!@#$%^&*()-_=+[]{};:'\" <>,./?"));
    }

    #[test]
    pub fn test_norm_slugs() {
        assert_eq!(
            "artist-colon-rainbow-dash-dash+super",
            sluggify("artist:rainbow-dash super")
        );
        assert_eq!("underscore_tag", sluggify("underscore_tag"));
    }
}
