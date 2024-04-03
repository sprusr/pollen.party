use axum::{routing::get, Router};

async fn placeholder() -> &'static str {
    "pollen is coming"
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new().route("/", get(placeholder));

    Ok(router.into())
}
