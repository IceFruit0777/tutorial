use crate::helper::spawn_app;

#[tokio::test]
async fn valid_subscribe() {
    let (address, pool) = spawn_app().await;
    let client = reqwest::Client::new();

    let body = "name=IceFruit%20huang&email=git%40github.com";
    let res = client
        .post(format!("{address}/subscribe"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request.");
    assert_eq!(200, res.status().as_u16());

    let data = sqlx::query!("select name, email from subscription")
        .fetch_one(pool.get_ref())
        .await
        .expect("failed to execute query.");
    dbg!(&data);
    assert_eq!("IceFruit huang", data.name);
    assert_eq!("git@github.com", data.email);
}

#[tokio::test]
async fn invalid_subscribe() {
    let (address, _) = spawn_app().await;
    let client = reqwest::Client::new();

    let datas = [
        ("name=&email=git%40github01.com", "name is empty."),
        ("name=IceFruit%20huang&email=", "email is empty."),
    ];
    for (body, payload) in datas {
        let res = client
            .post(format!("{address}/subscribe"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request.");
        assert_eq!(400, res.status().as_u16(), "{payload}");
    }
}
