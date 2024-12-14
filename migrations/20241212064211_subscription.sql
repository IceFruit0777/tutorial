CREATE TABLE subscription (
    id uuid NOT NULL,
    PRIMARY KEY (id),
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    subscribed_at timestamptz NOT NULL
);
