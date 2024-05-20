use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use chrono::{Local, Locale, NaiveTime};
use chrono_tz::Tz;
use reqwest::{header, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{cmp::min, str::FromStr, sync::Arc};

use crate::{
    html::{forecast, home, page},
    phone::get_phone_text,
    silam::Pollen,
    AppState,
};

const DECIMAL_PLACES: usize = 2;

#[derive(Deserialize)]
pub struct IndexParams {
    lon: Option<f32>,
    lat: Option<f32>,
    loc: Option<String>,
}

pub async fn index(
    Query(params): Query<IndexParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    if let Some(loc) = params.loc {
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

    let mut headers = HeaderMap::new();

    if let IndexParams {
        lon: Some(lon),
        lat: Some(lat),
        ..
    } = params
    {
        let (rounded_lon, rounded_lat) = (
            format!("{:.1$}", lon, DECIMAL_PLACES),
            format!("{:.1$}", lat, DECIMAL_PLACES),
        );

        if rounded_lon.parse::<f32>().unwrap() != lon || rounded_lat.parse::<f32>().unwrap() != lat
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
        let location_heading = format!(
            "{}, {}, {}, {} ({:.6$}, {:.6$})",
            location.name, location.admin1, location.admin2, location.cc, lat, lon, DECIMAL_PLACES,
        );

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

        let locale = Locale::from_str("en_GB").unwrap();

        let body = page(
            true,
            state.silam.read().unwrap().fetch_time,
            forecast(&pollen, &location_heading, &tz, &locale),
        );

        let max_age = get_max_age(&state.silam.read().unwrap().time_until_stale(), &tz);
        let cache_control = format!("s-max-age={}, public, immutable, must-revalidate", max_age);
        headers.insert(header::CACHE_CONTROL, cache_control.parse().unwrap());

        return (headers, body).into_response();
    }

    let body = page(false, state.silam.read().unwrap().fetch_time, home());

    let cache_control = format!(
        "s-max-age={}, public, immutable, must-revalidate",
        &state.silam.read().unwrap().time_until_stale().num_seconds()
    );
    headers.insert(header::CACHE_CONTROL, cache_control.parse().unwrap());

    (headers, body).into_response()
}

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

#[derive(Deserialize)]
pub struct ApiParams {
    lon: Option<f32>,
    lat: Option<f32>,
}

#[derive(Serialize)]
pub struct ApiError {
    msg: String,
}

#[derive(Serialize)]
pub struct ApiResponse {
    attribution: String,
    location: String,
    pollen: Vec<Pollen>,
}

pub async fn api(Query(params): Query<ApiParams>, State(state): State<Arc<AppState>>) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());

    if let ApiParams {
        lon: Some(lon),
        lat: Some(lat),
        ..
    } = params
    {
        let (rounded_lon, rounded_lat) = (
            format!("{:.1$}", lon, DECIMAL_PLACES),
            format!("{:.1$}", lat, DECIMAL_PLACES),
        );

        if rounded_lon.parse::<f32>().unwrap() != lon || rounded_lat.parse::<f32>().unwrap() != lat
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    msg: format!(
                        "Coordinates accept maximum {} decimal places",
                        DECIMAL_PLACES
                    ),
                }),
            )
                .into_response();
        }

        let location = state
            .reverse_geocoder
            .search((lat.into(), lon.into()))
            .record;
        let location_string = format!(
            "{}, {}, {}, {}",
            location.name, location.admin1, location.admin2, location.cc,
        );

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

        let max_age = get_max_age(&state.silam.read().unwrap().time_until_stale(), &tz);
        let cache_control = format!("s-max-age={}, public, immutable, must-revalidate", max_age);
        headers.insert(header::CACHE_CONTROL, cache_control.parse().unwrap());

        return (
            headers,
            Json(ApiResponse {
                attribution: "Data from FMI SILAM and EAN".to_string(),
                location: location_string,
                pollen,
            }),
        )
            .into_response();
    }

    let cache_control = format!(
        "s-max-age={}, public, immutable, must-revalidate",
        &state.silam.read().unwrap().time_until_stale().num_seconds()
    );
    headers.insert(header::CACHE_CONTROL, cache_control.parse().unwrap());

    (
        StatusCode::BAD_REQUEST,
        headers,
        Json(ApiError {
            msg: "?lat=&lon= query params missing".to_string(),
        }),
    )
        .into_response()
}

pub async fn emf_phone(State(state): State<Arc<AppState>>) -> Response {
    let lon: f32 = -2.38;
    let lat: f32 = 52.04;

    let tz: Tz = state
        .finder
        .get_tz_name(lon.into(), lat.into())
        .parse()
        .unwrap();

    let now_index: usize = (Local::now().with_timezone(&tz).to_utc()
        - state.silam.read().unwrap().start_time)
        .num_hours()
        .try_into()
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

    let text = get_phone_text(&pollen, now_index, tz);

    return Json(json!([
        {
            "verb": "say",
            "text": text
        },
        {
            "verb": "hangup"
        }
    ]))
    .into_response();
}
