use reqwest::Response;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helper::spawn_app;

async fn request(address: &str, body: &'static str) -> Response {
    let client = reqwest::Client::new();
    client
        .post(&format!("{address}/subscribe"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request.")
}

#[tokio::test]
async fn valid_subscribe() {
    let (address, pool, email_server) = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&email_server)
        .await;

    let body = "name=IceFruit%20huang&email=git%40github.com";
    let res = request(&address, body).await;
    assert_eq!(200, res.status().as_u16());

    let email_request = &email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };
    let text_link = get_link(&body["TextBody"].as_str().unwrap());
    let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
    assert_eq!(text_link, html_link);

    let record = sqlx::query!("select name, email, status from subscription")
        .fetch_one(pool.get_ref())
        .await
        .expect("failed to execute query.");
    assert_eq!("IceFruit huang", record.name);
    assert_eq!("git@github.com", record.email);
    assert_eq!("pending_confirmation", record.status);
}

#[tokio::test]
async fn invalid_subscribe() {
    let (address, _, _) = spawn_app().await;

    let datas = [
        ("name=&email=git%40github01.com", "name is empty."),
        ("name=IceFruit%20huang&email=", "email is empty."),
    ];
    for (body, payload) in datas {
        let res = request(&address, body).await;
        assert_eq!(400, res.status().as_u16(), "{payload}");
    }
}
