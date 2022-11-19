#![forbid(unsafe_code)]

mod get_thread;

use axum::{
    extract::{Extension, Form, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use minijinja::{Environment, context};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use get_thread::load_thread;

#[tokio::main]
async fn main() {
    let mut env: Environment<'static> = Environment::new();
    env.add_template("base.html", include_str!("templates/base.html")).unwrap();
    env.add_template("index.html", include_str!("templates/index.html")).unwrap();
    env.add_template("thread.html", include_str!("templates/thread.html")).unwrap();
    env.add_template("error.html", include_str!("templates/error.html")).unwrap();
    let env = Arc::new(env);

    let client = reqwest::Client::new();

    let app = Router::new()
        .route("/", get(form))
        .route("/", post(target))
        .route("/thread", get(thread))
        .layer(Extension(env))
        .layer(Extension(client));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn form(Extension(env): Extension<Arc<Environment<'static>>>) -> Html<String> {
    let tmpl = env.get_template("index.html").unwrap();
    Html(tmpl.render(context!()).unwrap())
}

#[derive(Deserialize)]
struct Input {
    url: String,
}

async fn target(Form(input): Form<Input>) -> Redirect {
    Redirect::to(&format!(
        "/thread?url={}",
        utf8_percent_encode(&input.url, NON_ALPHANUMERIC),
    ))
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            &Error::Reqwest(ref e) => write!(f, "{}", e),
            &Error::Other(ref e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            &Error::Reqwest(ref e) => Some(e),
            &Error::Other(_) => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::Reqwest(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Error {
        Error::Other("Invalid JSON".into())
    }
}

async fn thread(
    Extension(env): Extension<Arc<Environment<'static>>>,
    Extension(client): Extension<reqwest::Client>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let error = |code, msg| {
        let tmpl = env.get_template("error.html").unwrap();
        (
            code,
            Html(tmpl.render(context!(error => msg)).unwrap()),
        ).into_response()
    };

    let Some(url) = params.get("url") else {
        return error(StatusCode::NOT_FOUND, "No URL provided");
    };

    let thread = match load_thread(client, url).await {
        Ok(t) => t,
        Err(e) => return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("{}", e),
        ),
    };

    let tmpl = env.get_template("thread.html").unwrap();
    Html(tmpl.render(context!()).unwrap()).into_response()
}
