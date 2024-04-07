use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use chrono::{Local, Locale, NaiveTime};
use chrono_tz::Tz;
use maud::{html, DOCTYPE};
use nominatim::{Client, IdentificationMethod};
use reverse_geocoder::ReverseGeocoder;
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

use crate::silam::{Pollen, Silam};

struct AppState {
    finder: DefaultFinder,
    nominatim: Client,
    reverse_geocoder: ReverseGeocoder,
    silam: RwLock<Silam>,
}

#[derive(Deserialize)]
struct Params {
    lon: Option<f32>,
    lat: Option<f32>,
    loc: Option<String>,
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let state = Arc::new(AppState {
        finder: DefaultFinder::new(),
        nominatim: Client::new(IdentificationMethod::from_user_agent("pollen.party")),
        reverse_geocoder: ReverseGeocoder::new(),
        silam: RwLock::new(Silam::fetch().await.unwrap()),
    });

    let router = Router::new()
        .route("/", get(index))
        .with_state(Arc::clone(&state))
        .fallback_service(ServeDir::new("assets"));

    tokio::spawn(silam_refetch_if_stale(Arc::clone(&state)));

    Ok(router.into())
}

const DECIMAL_PLACES: usize = 2;

async fn index(Query(params): Query<Params>, State(state): State<Arc<AppState>>) -> Response {
    let result: Option<(Vec<Pollen>, String, Tz)> = match params {
        Params {
            lon: Some(lon),
            lat: Some(lat),
            ..
        } => {
            if (format!("{},{}", lon, lat) != format!("{:.2$},{:.2$}", lon, lat, DECIMAL_PLACES)) {
                return Redirect::to(&format!(
                    "/?lat={:.2$}&lon={:.2$}",
                    lat, lon, DECIMAL_PLACES
                ))
                .into_response();
            }

            let location = state
                .reverse_geocoder
                .search((lat.into(), lon.into()))
                .record;
            let tz: Tz = state
                .finder
                .get_tz_name(lon.into(), lat.into())
                .parse()
                .unwrap();
            let start_index: usize = (Local::now()
                .with_timezone(&tz)
                .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .unwrap()
                .to_utc()
                - state.silam.read().unwrap().start_time)
                .num_hours()
                .try_into()
                .unwrap();
            let end_index = start_index + 72;
            let pollen = state
                .silam
                .read()
                .unwrap()
                .get_at_coords(&lon, &lat)
                .drain(start_index..end_index)
                .collect();

            Some((
                pollen,
                format!(
                    "{}, {}, {}, {} ({:.6$}, {:.6$})",
                    location.name,
                    location.admin1,
                    location.admin2,
                    location.cc,
                    lat,
                    lon,
                    DECIMAL_PLACES,
                ),
                tz,
            ))
        }
        Params { loc: Some(loc), .. } => {
            let nominatim_response = state.nominatim.search(&loc).await.unwrap();
            let place = nominatim_response.first().unwrap();
            return Redirect::to(&format!(
                "/?lat={:.2$}&lon={:.2$}",
                place.lat.parse::<f32>().unwrap(),
                place.lon.parse::<f32>().unwrap(),
                DECIMAL_PLACES,
            ))
            .into_response();
        }
        _ => None,
    };

    let locale = Locale::from_str("en_GB").unwrap();

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "pollen.party" }
                meta name="viewport" content="width=device-width, initial-scale=1" {}
                link  rel="stylesheet" href="style.css" {}
                script src="script.js" defer {}
            }
            body {
                header {
                    h1 { "⚘ " a href="/" { "pollen.party" } " ⚘" }
                }
                main {
                    @if let Some((pollen, location, timezone)) = result {
                        h2 { (location) }
                        table {
                            @for p in &pollen {
                                tr {
                                    td { (p.time.with_timezone(&timezone).format_localized("%a %R", locale)) }
                                    td { (p.pollen_index) " (" (p.pollen_index_source) ")" }
                                }
                            }
                        }
                    } @else {
                        p {
                            "This website provides pollen forecasts for Europe. "
                            "Times displayed in location's local timezone. Data from "
                            a href="https://silam.fmi.fi/" { "FMI SILAM" }
                            " and "
                            a href="https://www.polleninfo.org/" { "EAN" }
                            ". Search uses "
                            a href="https://www.openstreetmap.org/copyright" { "OpenStreetMap" }
                            "."
                        }
                        form action="" method="GET" id="geo-form" {
                            input type="button" value="Use my location" id="geo" class="big";
                            input type="hidden" name="lat" id="lat";
                            input type="hidden" name="lon" id="lon";
                        }
                        p class="center" { "or" }
                        form action="" method="GET" {
                            label for="loc" { "Location" }
                            input type="text" name="loc" id="loc" placeholder="E.g. Helsinki, Finland";
                            input type="submit" value="Search";
                        }
                    }
                }
                footer {
                    p {
                        small {
                            "Data was fetched at: "
                            (state.silam.read().unwrap().fetch_time)
                            ". For enquiries contact webmaster at this domain."
                        }
                    }
                }
            }
        }
    }.into_response()
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
