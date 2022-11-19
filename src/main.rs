#![forbid(unsafe_code)]

use axum::{
    extract::{Extension, Form, Query},
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

#[tokio::main]
async fn main() {
    let mut env: Environment<'static> = Environment::new();
    env.add_template("base.html", include_str!("templates/base.html")).unwrap();
    env.add_template("index.html", include_str!("templates/index.html")).unwrap();
    env.add_template("thread.html", include_str!("templates/thread.html")).unwrap();
    let env = Arc::new(env);

    let app = Router::new()
        .route("/", get(form))
        .route("/", post(target))
        .route("/thread", get(thread))
        .layer(Extension(env));

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

async fn thread(
    Extension(env): Extension<Arc<Environment<'static>>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let Some(url) = params.get("url") else {
        return Redirect::permanent("/").into_response();
    };

    let tmpl = env.get_template("thread.html").unwrap();
    Html(tmpl.render(context!(url => url)).unwrap()).into_response()
}
