CREATE TABLE newsletter_issue (
    newsletter_issue_id uuid NOT NULL,
    subject TEXT NOT NULL,
    text_body TEXT NOT NULL,
    html_body TEXT NOT NULL,
    published_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(newsletter_issue_id)
)
