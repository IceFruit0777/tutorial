pub enum SubscriberStatus {
    PendingConfirmation,
    Confirmed,
}

impl SubscriberStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SubscriberStatus::PendingConfirmation => "pending_confirmation",
            SubscriberStatus::Confirmed => "confirmed",
        }
    }
}
