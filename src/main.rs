use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use chrono::{Local, Locale, NaiveTime};
use chrono_tz::Tz;
use maud::{html, DOCTYPE};
use nominatim::{Client, IdentificationMethod};
use reqwest::header;
use reverse_geocoder::ReverseGeocoder;
use serde::Deserialize;
use shuttle_runtime::SecretStore;
use std::{
    cmp::min,
    str::FromStr,
    sync::{Arc, RwLock},
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
    silam_email: Option<String>,
}

#[derive(Deserialize)]
struct Params {
    lon: Option<f32>,
    lat: Option<f32>,
    loc: Option<String>,
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_axum::ShuttleAxum {
    let silam_email = secrets.get("SILAM_EMAIL");

    let state = Arc::new(AppState {
        finder: DefaultFinder::new(),
        nominatim: Client::new(IdentificationMethod::from_user_agent("pollen.party")),
        reverse_geocoder: ReverseGeocoder::new(),
        silam: RwLock::new(Silam::fetch(&silam_email).await.unwrap()),
        silam_email,
    });

    let router = Router::new()
        .route("/", get(index))
        .with_state(Arc::clone(&state))
        .fallback_service(ServeDir::new("assets"));

    tokio::spawn(silam_refetch_if_stale(Arc::clone(&state)));

    Ok(router.into())
}

const DECIMAL_PLACES: usize = 2;

fn get_max_age(time_until_stale: &chrono::Duration, tz: &Tz) -> i64 {
    let seconds_until_stale = time_until_stale.num_seconds();
    let now = Local::now().with_timezone(tz);
    let time_until_local_midnight = (now + chrono::Duration::days(1))
        .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap()
        - now;
    let seconds_until_local_midnight = time_until_local_midnight.num_seconds();
    min(seconds_until_stale, seconds_until_local_midnight)
}

async fn index(Query(params): Query<Params>, State(state): State<Arc<AppState>>) -> Response {
    let result: Option<(Vec<Pollen>, String, Tz)> = match params {
        Params {
            lon: Some(lon),
            lat: Some(lat),
            ..
        } => {
            let (rounded_lon, rounded_lat) = (
                format!("{:.1$}", lon, DECIMAL_PLACES),
                format!("{:.1$}", lat, DECIMAL_PLACES),
            );

            if rounded_lon.parse::<f32>().unwrap() != lon
                || rounded_lat.parse::<f32>().unwrap() != lat
            {
                return Redirect::permanent(&format!(
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
            let nominatim_response = match state.nominatim.search(&loc).await {
                Ok(res) => res,
                Err(_) => return Redirect::temporary("/").into_response(),
            };
            let place = match nominatim_response.first() {
                Some(first) => first,
                None => return Redirect::temporary("/").into_response(),
            };
            return Redirect::permanent(&format!(
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

    let cache_control = match result {
        Some((_, _, tz)) => {
            let max_age = get_max_age(&state.silam.read().unwrap().time_until_stale(), &tz);
            format!("s-max-age={}, public, immutable, must-revalidate", max_age)
        }
        None => format!(
            "s-max-age={}, public, immutable, must-revalidate",
            &state.silam.read().unwrap().time_until_stale().num_seconds()
        ),
    };
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, cache_control.parse().unwrap());

    let body = html! {
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
                    @if result.is_some() {
                        p { small { a href="/" { "‹ Back" } } }
                    }
                }
                main {
                    @if let Some((pollen, location, timezone)) = result {
                        h2 { (location) }
                        p {
                            "Data from "
                            a href="https://silam.fmi.fi/" { "FMI SILAM" }
                            " and "
                            a href="https://www.polleninfo.org/" { "EAN" }
                            "."
                        }
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
                            input type="text" name="loc" id="loc" placeholder="E.g. Helsinki, Finland" required;
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
    };

    (headers, body).into_response()
}

async fn silam_refetch_if_stale(state: Arc<AppState>) -> () {
    let mut interval = time::interval(std::time::Duration::from_secs(10));
    loop {
        interval.tick().await;
        if state.silam.read().unwrap().is_stale() {
            let silam = Silam::fetch(&state.silam_email).await.unwrap();
            *state.silam.write().unwrap() = silam;
        }
    }
}
