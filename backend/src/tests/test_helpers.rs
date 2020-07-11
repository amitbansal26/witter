#![allow(dead_code)]

mod test_db;

use crate::Server;
use crate::State;
use crate::{make_db_pool, server};
use futures::{executor::block_on, prelude::*};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::prelude::Connect;
use sqlx::PgConnection;
use sqlx::PgPool;
use sqlx::Postgres;
use std::collections::HashMap;
use std::env;
use std::pin::Pin;
use test_db::TestDb;

pub use assert_json_diff::{assert_json_eq, assert_json_include};
pub use serde_json::{json, Value};
pub use shared::payloads::*;
pub use shared::responses::*;
pub use tide::http::headers::HeaderName;
pub use tide::http::Request;
pub use tide::http::Response;
pub use tide::http::{Method, Url};

pub async fn test_setup() -> TestServer {
    std::env::set_var("APP_ENV", "test");
    dotenv::dotenv().ok();
    pretty_env_logger::try_init().ok();

    let test_db = TestDb::new().await;
    let db_pool = test_db.db();

    let server = server(db_pool).await;
    TestServer::new(server, test_db)
}

pub struct TestServer {
    service: Server<State>,
    test_db: TestDb,
}

impl TestServer {
    fn new(service: Server<State>, test_db: TestDb) -> Self {
        Self { service, test_db }
    }

    pub async fn simulate(&mut self, req: Request) -> tide::Result<Response> {
        self.service.respond(req).await
    }
}

pub trait BodyJson {
    fn body_json<T: DeserializeOwned>(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<T, Box<dyn std::error::Error>>>>>;
}

impl BodyJson for Response {
    fn body_json<T: DeserializeOwned>(
        mut self,
    ) -> Pin<Box<dyn Future<Output = Result<T, Box<dyn std::error::Error>>>>> {
        Box::pin(async move {
            let body = self.body_string().await?;
            dbg!(&body);
            Ok(serde_json::from_str(&body)?)
        })
    }
}

pub fn get(url: &str) -> TestRequest {
    TestRequest {
        url: url.to_string(),
        headers: HashMap::new(),
        kind: TestRequestKind::Get,
    }
}

pub fn post<T: Serialize>(url: &str, body: Option<T>) -> TestRequest {
    let body = body.map(|body| serde_json::to_value(body).unwrap());
    let kind = TestRequestKind::Post(body);

    TestRequest {
        url: url.to_string(),
        headers: HashMap::new(),
        kind,
    }
}

pub fn empty_post(url: &str) -> TestRequest {
    post(url, None::<()>)
}

#[derive(Debug)]
pub struct TestRequest {
    url: String,
    headers: HashMap<String, String>,
    kind: TestRequestKind,
}

#[derive(Debug)]
pub enum TestRequestKind {
    Get,
    Post(Option<Value>),
}

impl TestRequest {
    pub async fn send(self, server: &mut TestServer) -> Response {
        let url = Url::parse(&format!("http://example.com{}", self.url)).unwrap();

        let mut req = match self.kind {
            TestRequestKind::Get => Request::new(Method::Get, url),
            TestRequestKind::Post(body) => {
                let mut req = Request::new(Method::Post, url);
                if let Some(body) = body {
                    req.set_body(body.to_string());
                    req.set_content_type("application/json".parse().unwrap());
                }
                req
            }
        };

        for (key, value) in self.headers {
            req.append_header(key.as_str(), value.as_str());
        }

        server.simulate(req).await.unwrap()
    }

    pub fn header(mut self, key: &str, value: impl ToString) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }
}

pub async fn create_user_and_authenticate(
    server: &mut TestServer,
    username: Option<String>,
) -> TokenResponse {
    let resp = post(
        "/users",
        Some(CreateUserPayload {
            username: username.unwrap_or_else(|| "bob".to_string()),
            password: "foobar".to_string(),
        }),
    )
    .send(server)
    .await;
    assert_eq!(resp.status(), 201);

    let resp = resp
        .body_json::<ApiResponse<TokenResponse>>()
        .await
        .unwrap();
    resp.data
}
