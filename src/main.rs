use axum::{routing::get, Router};
use maud::{html, Markup, DOCTYPE};

async fn placeholder() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "pollen.party" }
            }
            body {
                p { "pollen is coming" }
            }
        }
    }
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new().route("/", get(placeholder));

    Ok(router.into())
}
