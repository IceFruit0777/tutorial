use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: &str) -> Result<SubscriberEmail, String> {
        if s.validate_email() {
            Ok(Self(s.into()))
        } else {
            tracing::error!("`{s}` is not a valid subscriber email.");
            Err(format!("`{s}` is not a valid subscriber email."))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claim::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use rand::{rngs::StdRng, SeedableRng};

    use crate::domain::SubscriberEmail;

    // --------单元测试SubscriberEmail start--------
    #[derive(Clone, Debug)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);

            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_email(email: ValidEmailFixture) -> bool {
        // dbg!(&email.0);
        SubscriberEmail::parse(&email.0).is_ok()
    }

    #[test]
    fn invalid_empty_email() {
        let email = "";
        assert_err!(SubscriberEmail::parse(email));

        let email = " ";
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn invalid_missing_at_symbol_email() {
        let email = "gitgithub.com";
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn invalid_missing_subject_email() {
        let email = "@github.com";
        assert_err!(SubscriberEmail::parse(email));
    }
}
