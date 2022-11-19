#![forbid(unsafe_code)]

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

#[derive(Deserialize)]
struct ActivityPubToot {
    id: String,
    published: String,
    attributedTo: String,
    content: String,
}

struct Toot {
    url: String,
    author: String,
    message: String,
}

impl From<ActivityPubToot> for Toot {
    fn from(toot: ActivityPubToot) -> Toot {
        Toot {
            url: toot.id,
            author: toot.attributedTo,
            message: toot.content,
        }
    }
}

struct Thread {
    toot: Toot,
    children: Vec<Thread>,
}

#[derive(Debug)]
enum Error {
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
    fn from(e: serde_json::Error) -> Error {
        Error::Other("Invalid JSON".into())
    }
}

async fn load_thread(client: reqwest::Client, target_url: &str) -> Result<Thread, Error> {
    // Load the provided toot
    eprintln!("Getting toot {}", target_url);
    let res = client
        .get(target_url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    // Get that first toot
    let toot: ActivityPubToot = serde_json::from_value(res.clone())?;
    let toot = toot.into();

    // Get the replies URL
    let page_url = res
        .get("replies")
        .and_then(|r| r.get("first"))
        .and_then(|r| r.get("next"));
    let mut page_url = match page_url {
        Some(serde_json::Value::String(s)) => s.to_owned(),
        _ => return Err(Error::Other("Missing replies link".into())),
    };

    let mut thread = Thread {
        toot,
        children: Vec::new(),
    };

    // Load the replies, which might spawn multiple pages
    loop {
        eprintln!("Getting page of replies {}", page_url);
        let mut res = client
            .get(page_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let Some(serde_json::Value::Array(items)) = res
            .get_mut("items")
            .map(serde_json::Value::take)
        else {
            return Err(Error::Other("Invalid replies data".into()));
        };

        for item in items {
            eprintln!("Reading item");
            if let serde_json::Value::String(_) = item {
                // Skip
                eprintln!("Is URL, skip");
                continue;
            }
            let item: ActivityPubToot = serde_json::from_value(item)?;
            thread.children.push(Thread {
                toot: item.into(),
                children: Vec::new(),
            });
        }

        match res.get("next") {
            Some(serde_json::Value::String(url)) => page_url = url.to_owned(),
            _ => break,
        }
    }

    eprintln!("Done getting replies");
    Ok(thread)
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
