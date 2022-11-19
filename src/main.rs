#![forbid(unsafe_code)]

use axum::{
    extract::Extension,
    routing::get,
    Router,
};
use minijinja::{Environment, context};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let mut env: Environment<'static> = Environment::new();
    env.add_template("base.html", include_str!("templates/base.html")).unwrap();
    env.add_template("index.html", include_str!("templates/index.html")).unwrap();
    let env = Arc::new(env);

    let app = Router::new()
        .route("/", get(root))
        .layer(Extension(env));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root(Extension(env): Extension<Arc<Environment<'static>>>) -> String {
    let tmpl = env.get_template("index.html").unwrap();
    tmpl.render(context!()).unwrap()
}
