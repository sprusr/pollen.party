use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use chrono_tz::Tz;
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tzf_rs::DefaultFinder;

mod silam;

use crate::silam::Silam;

struct AppState {
    silam: Silam,
    finder: DefaultFinder,
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let state = Arc::new(AppState {
        silam: Silam::fetch().await.unwrap(),
        finder: DefaultFinder::new(),
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

    let timezone_name = state.finder.get_tz_name(lon as f64, lat as f64);
    let timezone: Tz = timezone_name.parse().unwrap();

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "pollen.party" }
                link  rel="stylesheet" href="style.css" {}
                script src="script.js" defer {}
            }
            body {
                p { "pollen is coming" }
                p { (location) " info" }
                p { "time: " (pollen[0].time.with_timezone(&timezone)) " (local time at location)" }
                p { "index: " (pollen[0].pollen_index) }
                p { "main source: " (pollen[0].pollen_index_source) }
            }
        }
    }
}
