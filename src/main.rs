use std::sync::Arc;

use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;
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

#[derive(Deserialize)]
struct Params {
    lon: Option<f32>,
    lat: Option<f32>,
}

async fn index(Query(params): Query<Params>, State(state): State<Arc<AppState>>) -> Markup {
    let (location, lon, lat): (String, f32, f32) = match (params.lon, params.lat) {
        (Some(lon), Some(lat)) => (format!("{:?}", (lon, lat)), lon, lat),
        _ => ("Kaivopuisto".to_string(), 24.956100, 60.156136),
    };

    let pollen = state.silam.get_at_coords(&lon, &lat);

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
                p { (location) " info" }
                p { "time: " (pollen[0].time) }
                p { "index: " (pollen[0].pollen_index) }
                p { "main source: " (pollen[0].pollen_index_source) }
            }
        }
    }
}
