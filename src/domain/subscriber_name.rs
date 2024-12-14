use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<SubscriberName, String> {
        let is_empty = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;
        let forbidden_characters = ['/', '(', ')', '{', '}', '"', '<', '>', '\\'];
        let contain_fb_chars = s.chars().any(|c| forbidden_characters.contains(&c));

        if is_empty || is_too_long || contain_fb_chars {
            return Err(format!("`{s}` is not a valid subscriber name."));
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};

    use crate::domain::SubscriberName;

    #[test]
    fn invalid_empty_name() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));

        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn valid_max_length_name() {
        let name = "a".repeat(256);
        assert_ok!(SubscriberName::parse(name));

        let name = "ă".repeat(256);
        assert_ok!(SubscriberName::parse(name));

        let name = "我".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn invalid_length_name() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));

        let name = "ă".repeat(257);
        assert_err!(SubscriberName::parse(name));

        let name = "我".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn invalid_forbidden_characters() {
        let forbidden_characters = ['/', '(', ')', '{', '}', '"', '<', '>', '\\'];
        for c in forbidden_characters.iter() {
            let name = c.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }
}
