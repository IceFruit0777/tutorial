use crate::routes::FormData;

use super::{SubscriberEmail, SubscriberName};

pub struct Subscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
}

impl TryFrom<FormData> for Subscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;

        Ok(Self { name, email })
    }
}
