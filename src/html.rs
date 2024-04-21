use chrono::{DateTime, Locale, Utc};
use chrono_tz::Tz;
use maud::{html, Markup, DOCTYPE};

use crate::silam::Pollen;

pub fn page(back_enabled: bool, fetched_at: DateTime<Utc>, content: Markup) -> Markup {
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
                    @if back_enabled {
                        p { small { a href="/" { "‹ Back" } } }
                    }
                }
                main {
                    (content)
                }
                footer {
                    p {
                        small {
                            "Data was fetched at: "
                            (fetched_at)
                            ". For enquiries contact webmaster at this domain."
                        }
                    }
                }
            }
        }
    }
}

pub fn home() -> Markup {
    html! {
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

pub fn forecast(pollen: &Vec<Pollen>, location: &String, timezone: &Tz, locale: &Locale) -> Markup {
    html! {
        h2 { (location) }
        p {
            "Pollen count: 1 (low) - 5 (high). Main pollen source in brackets. Data from "
            a href="https://silam.fmi.fi/" { "FMI SILAM" }
            " and "
            a href="https://www.polleninfo.org/" { "EAN" }
            "."
        }
        table {
            tr {
                td {}
                td { (pollen[0].time.with_timezone(timezone).format_localized("%a", *locale)) }
                td { (pollen[24].time.with_timezone(timezone).format_localized("%a", *locale)) }
                td { (pollen[48].time.with_timezone(timezone).format_localized("%a", *locale)) }
            }
            @for n in 0..24 {
                tr {
                    td { (pollen[n].time.with_timezone(timezone).format_localized("%R", *locale)) }
                    td class={ "level-" (pollen[n].pollen_index) } { (pollen[n].pollen_index) " (" (pollen[n].pollen_index_source) ")" }
                    td class={ "level-" (pollen[n + 24].pollen_index) } { (pollen[n + 24].pollen_index) " (" (pollen[n + 24].pollen_index_source) ")" }
                    td class={ "level-" (pollen[n + 48].pollen_index) } { (pollen[n + 48].pollen_index) " (" (pollen[n + 48].pollen_index_source) ")" }
                }
            }
        }
    }
}
