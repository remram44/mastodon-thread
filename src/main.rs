#![forbid(unsafe_code)]

mod thread;

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
use std::time::Duration;

use thread::load_thread;

#[tokio::main]
async fn main() {
    let mut env: Environment<'static> = Environment::new();
    env.add_template("base.html", include_str!("templates/base.html")).unwrap();
    env.add_template("index.html", include_str!("templates/index.html")).unwrap();
    env.add_template("thread.html", include_str!("templates/thread.html")).unwrap();
    env.add_template("error.html", include_str!("templates/error.html")).unwrap();
    let env = Arc::new(env);

    let client = reqwest::Client::builder()
        .user_agent(format!("{} (+{})", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_HOMEPAGE")))
        .timeout(Duration::new(10, 0))
        .build().unwrap();

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

    let thread = if true {
        match load_thread(client, url).await {
            Ok(t) => t,
            Err(e) => return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("{}", e),
            ),
        }
    } else {
        use thread::{Reply, Thread, Toot};
        use std::sync::Mutex;
        let f = |u: &str, a: &str, m: &str, c: Vec<Reply>| { Arc::new(Mutex::new(Thread {
            toot: Toot {
                url: u.to_owned(),
                author: a.to_owned(),
                message: m.to_owned(),
            },
            children: c,
        })) };
        let t = |u: &str, a: &str, m: &str, c: Vec<Reply>| { Reply::Thread(f(u, a, m, c)) };
        f("https://mastodon.social/users/austinkocher/statuses/109349270891780546", "https://mastodon.social/users/austinkocher", "<p>Evan Prodromou, who helped inspire &amp; create the <a href=\"https://mastodon.social/tags/ActivityPub\" rel=\"noopener noreferrer\">#<span>ActivityPub</span></a> protocol that makes <span><a href=\"https://mastodon.social/@Mastodon\" rel=\"noopener noreferrer\">@<span>Mastodon</span></a></span> run, called this recent <a href=\"https://mastodon.social/tags/Twittermigration\" rel=\"noopener noreferrer\">#<span>Twittermigration</span></a> a “tipping point”.</p><p>“I have not been this excited about federated social networks since we published ActivityPub. The shift has started. … We’re doing this. Don’t be on the wrong side of history.” <span><a href=\"https://prodromou.pub/@evan\" rel=\"noopener noreferrer\">@<span>evan</span></a></span></p><p>Blog post here: <a href=\"https://evanp.me/2022/11/11/its-happening/?utm_source=thenewstack&amp;utm_medium=website&amp;utm_content=inline-mention&amp;utm_campaign=platform\" rel=\"noopener noreferrer\"><span>https://</span><span>evanp.me/2022/11/11/its-happen</span><span>ing/?utm_source=thenewstack&amp;utm_medium=website&amp;utm_content=inline-mention&amp;utm_campaign=platform</span></a></p>", vec![
            t("https://fosstodon.org/users/RobLoach/statuses/109349495415864020", "https://fosstodon.org/users/RobLoach", "<p><span><a href=\"https://mastodon.social/@austinkocher\" rel=\"noopener noreferrer\">@<span>austinkocher</span></a></span> <span><a href=\"https://mastodon.social/@Mastodon\" rel=\"noopener noreferrer\">@<span>Mastodon</span></a></span> <span><a href=\"https://prodromou.pub/@evan\" rel=\"noopener noreferrer\">@<span>evan</span></a></span> ItsHappening.gif</p>", vec![]),
            t("https://mastodon.social/users/tagomago/statuses/109349852895789855", "https://mastodon.social/users/tagomago", "<p><span><a href=\"https://mastodon.social/@austinkocher\" rel=\"noopener noreferrer\">@<span>austinkocher</span></a></span> <span><a href=\"https://mastodon.social/@Mastodon\" rel=\"noopener noreferrer\">@<span>Mastodon</span></a></span> <span><a href=\"https://prodromou.pub/@evan\" rel=\"noopener noreferrer\">@<span>evan</span></a></span> Pretty nice to have him back here too!!</p>", vec![]),
            t("https://federate.social/users/maxb/statuses/109349905467831684", "https://federate.social/users/maxb", "<p><span><a href=\"https://mastodon.social/@austinkocher\" rel=\"noopener noreferrer\">@<span>austinkocher</span></a></span> <span><a href=\"https://mastodon.social/@Mastodon\" rel=\"noopener noreferrer\">@<span>Mastodon</span></a></span> <span><a href=\"https://prodromou.pub/@evan\" rel=\"noopener noreferrer\">@<span>evan</span></a></span> </p><p>Hopefully we can keep it from being \"monetized\" , commercialized or weaponized (propaganda). Keep vigilant and keep up the good work.</p>", vec![]),
            t("https://fosstodon.org/users/jvalleroy/statuses/109350111126208677", "https://fosstodon.org/users/jvalleroy", "<p><span><a href=\"https://mastodon.social/@austinkocher\" rel=\"noopener noreferrer\">@<span>austinkocher</span></a></span> <span><a href=\"https://mastodon.social/@Mastodon\" rel=\"noopener noreferrer\">@<span>Mastodon</span></a></span> <span><a href=\"https://prodromou.pub/@evan\" rel=\"noopener noreferrer\">@<span>evan</span></a></span> It's official then.</p>", vec![]),
            t("https://mas.to/users/Jakki/statuses/109350979959344823", "https://mas.to/users/Jakki", "<p><span><a href=\"https://mastodon.social/@austinkocher\" rel=\"noopener noreferrer\">@<span>austinkocher</span></a></span> <span><a href=\"https://mastodon.social/@Mastodon\" rel=\"noopener noreferrer\">@<span>Mastodon</span></a></span> <span><a href=\"https://prodromou.pub/@evan\" rel=\"noopener noreferrer\">@<span>evan</span></a></span> Thats SO cute! Everyone should get to see their dreams come true. It makes life worth living🖖</p>", vec![]),
            t("https://mastodon.star-one.org.uk/users/simon/statuses/109352823679228954", "https://mastodon.star-one.org.uk/users/simon", "<p><span><a href=\"https://prodromou.pub/@evan\" rel=\"noopener noreferrer\">@<span>evan</span></a></span> <span><a href=\"https://mastodon.social/@Mastodon\" rel=\"noopener noreferrer\">@<span>Mastodon</span></a></span> <span><a href=\"https://mastodon.social/@austinkocher\" rel=\"noopener noreferrer\">@<span>austinkocher</span></a></span> <span><a href=\"https://mastodon.social/@Gargron\" rel=\"noopener noreferrer\">@<span>Gargron</span></a></span> — This is all grand (it is), and I don’t want to burst the balloon of how good it is this place is taking off, but I do feel honour-bound to mention that Usenet, FidoNet, and various other public messaging platforms from 30+ years ago also worked on the decentralised / federated / ‘nobody owns it so nobody can take it away’ principles of ActivityPub / Mastodon</p>", vec![
                t("https://prodromou.pub/users/evan/statuses/109353682035030878", "https://prodromou.pub/users/evan", "<p><span><a href=\"https://mastodon.star-one.org.uk/@simon\" rel=\"noopener noreferrer\">@<span>simon</span></a></span> <span><a href=\"https://mastodon.social/@Mastodon\" rel=\"noopener noreferrer\">@<span>Mastodon</span></a></span> <span><a href=\"https://mastodon.social/@austinkocher\" rel=\"noopener noreferrer\">@<span>austinkocher</span></a></span> <span><a href=\"https://mastodon.social/@Gargron\" rel=\"noopener noreferrer\">@<span>Gargron</span></a></span></p><p>Your honour is intact.</p>", vec![]),
            ]),
        ])
    };

    let tmpl = env.get_template("thread.html").unwrap();
    Html(tmpl.render(context!(thread => thread)).unwrap()).into_response()
}
