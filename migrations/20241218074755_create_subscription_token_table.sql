CREATE TABLE subscription_token (
    subscription_token TEXT NOT NULL,
    PRIMARY KEY (subscription_token),
    subscriber_id uuid NOT NULL REFERENCES subscription(id)
)
