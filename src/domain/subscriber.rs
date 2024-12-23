use crate::routes::FormData;

use super::{SubscriberEmail, SubscriberName, SubscriberStatus};

pub struct Subscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
    pub status: SubscriberStatus,
}

impl TryFrom<FormData> for Subscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(&form.name)?;
        let email = SubscriberEmail::parse(&form.email)?;
        let status = SubscriberStatus::PendingConfirmation;

        Ok(Self {
            name,
            email,
            status,
        })
    }
}
