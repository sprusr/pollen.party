use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use chrono::Locale;
use chrono_tz::Tz;
use maud::{html, Markup, DOCTYPE};
use serde::Deserialize;
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::time;
use tower_http::services::ServeDir;
use tzf_rs::DefaultFinder;

mod silam;

use crate::silam::Silam;

struct AppState {
    silam: RwLock<Silam>,
    finder: DefaultFinder,
}

#[derive(Deserialize)]
struct Params {
    lon: Option<f32>,
    lat: Option<f32>,
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let state = Arc::new(AppState {
        silam: RwLock::new(Silam::fetch().await.unwrap()),
        finder: DefaultFinder::new(),
    });

    let router = Router::new()
        .route("/", get(index))
        .with_state(Arc::clone(&state))
        .fallback_service(ServeDir::new("assets"));

    tokio::spawn(silam_refetch_if_stale(Arc::clone(&state)));

    Ok(router.into())
}

async fn index(Query(params): Query<Params>, State(state): State<Arc<AppState>>) -> Markup {
    let (location, lon, lat): (String, f32, f32) = match (params.lon, params.lat) {
        (Some(lon), Some(lat)) => (format!("Unknown Location ({}, {})", lat, lon), lon, lat),
        _ => ("Kaivopuisto".to_string(), 24.956100, 60.156136),
    };

    let pollen = state.silam.read().unwrap().get_at_coords(&lon, &lat);

    let timezone_name = state.finder.get_tz_name(lon as f64, lat as f64);
    let timezone: Tz = timezone_name.parse().unwrap();

    let locale = Locale::from_str("en_GB").unwrap();

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
                h1 { "pollen.party" }
                p {
                    "This website provides pollen forecasts for Europe. Data from "
                    a href="https://silam.fmi.fi/" { "FMI SILAM" }
                    " and "
                    a href="https://www.polleninfo.org/" { "EAN" }
                    ". Times displayed in location's local timezone."
                }
                form action="" method="GET" id="form" {
                    label for="lat" { "Latitude" }
                    input type="text" value=(lat) name="lat" id="lat";
                    label for="lon" { "Longitude" }
                    input type="text" value=(lon) name="lon" id="lon";
                    input type="button" value="Geolocate" id="geolocate";
                    input type="submit";
                }
                h2 { (location) }
                table {
                    tr {
                        @for p in &pollen {
                            th { (p.time.with_timezone(&timezone).format_localized("%A %X", locale)) }
                        }
                    }
                    tr {
                        @for p in &pollen {
                            td { (p.pollen_index) " (" (p.pollen_index_source) ")" }
                        }
                    }
                }
                p {
                    small { "Data was fetched at: " (state.silam.read().unwrap().fetch_time) }
                }
            }
        }
    }
}

async fn silam_refetch_if_stale(state: Arc<AppState>) -> () {
    let mut interval = time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        if state.silam.read().unwrap().is_stale() {
            let silam = Silam::fetch().await.unwrap();
            *state.silam.write().unwrap() = silam;
        }
    }
}
