use axum::{routing::get, Router};
use nominatim::{Client, IdentificationMethod};
use reverse_geocoder::ReverseGeocoder;
use shuttle_runtime::SecretStore;
use std::sync::{Arc, RwLock};
use tokio::time;
use tower_http::services::ServeDir;
use tzf_rs::DefaultFinder;

mod handlers;
mod html;
mod silam;

use crate::{handlers::index, silam::Silam};

pub struct AppState {
    finder: DefaultFinder,
    nominatim: Client,
    reverse_geocoder: ReverseGeocoder,
    silam: RwLock<Silam>,
    silam_email: Option<String>,
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
