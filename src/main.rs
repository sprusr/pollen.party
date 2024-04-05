use std::sync::Arc;

use axum::{extract::State, routing::get, Router};
use maud::{html, Markup, DOCTYPE};
use tower_http::services::ServeDir;

use crate::silam::Silam;

mod silam;

struct AppState {
    silam: Silam,
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let state = Arc::new(AppState {
        silam: Silam::fetch().await.unwrap(),
    });

    let router = Router::new()
        .route("/", get(index))
        .with_state(state)
        .fallback_service(ServeDir::new("assets"));

    Ok(router.into())
}

async fn index(State(state): State<Arc<AppState>>) -> Markup {
    let lon = 24.956100;
    let lat = 60.156136;
    let pollen = state.silam.get_first_at_coords(&lon, &lat);

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "pollen.party" }
                link  rel="stylesheet" href="style.css";
            }
            body {
                p { "pollen is coming" }
                p { "kaivopuisto info" }
                p { "index when server started was: " (pollen.pollen_index) }
                p { "main source when server started was: " (pollen.pollen_index_source) }
            }
        }
    }
}
