use crate::tests::test_helpers::*;

#[async_std::test]
async fn logging_in_without_auth_header() {
    let mut server = test_setup().await;

    let res = post(
        "/users",
        CreateUserPayload {
            username: "bob".to_string(),
            password: "foobar".to_string(),
        },
    )
    .send(&mut server)
    .await;
    assert_eq!(res.status(), 201);

    let res = get("/me").send(&mut server).await;
    assert_eq!(res.status(), 400);

    let content_type = res
        .header("Content-Type".parse::<HeaderName>().unwrap())
        .unwrap()
        .get(0)
        .unwrap()
        .as_str();
    assert_eq!(content_type, "application/json");

    let json = res.body_json::<Value>().await.unwrap();

    assert_json_include!(
        actual: json,
        expected: json!({
            "error": {
                "message": "Missing value for `Authorization` header"
            }
        })
    );
}

#[async_std::test]
async fn logging_in_with_invalid_auth_header() {
    let mut server = test_setup().await;

    let res = post(
        "/users",
        CreateUserPayload {
            username: "bob".to_string(),
            password: "foobar".to_string(),
        },
    )
    .send(&mut server)
    .await;
    assert_eq!(res.status(), 201);

    let resp = res.body_json::<ApiResponse<TokenResponse>>().await.unwrap();
    let token = resp.data.token;

    let res = get("/me")
        .header("Authorization", format!("foo {}", token))
        .send(&mut server)
        .await;
    assert_eq!(res.status(), 400);
}

#[async_std::test]
async fn logging_in_with_unknown_user_gives_404() {
    let mut server = test_setup().await;

    let res = post("/users/bob/session", json!({ "password": "foobar" }))
        .send(&mut server)
        .await;
    assert_eq!(res.status(), 404);
}

#[async_std::test]
async fn logging_in_with_invalid_token() {
    let mut server = test_setup().await;

    let res = post(
        "/users",
        CreateUserPayload {
            username: "bob".to_string(),
            password: "foobar".to_string(),
        },
    )
    .send(&mut server)
    .await;
    assert_eq!(res.status(), 201);

    let res = post(
        "/users/bob/session",
        LoginPayload {
            password: "baz".to_string(),
        },
    )
    .send(&mut server)
    .await;
    assert_eq!(res.status(), 403);
}