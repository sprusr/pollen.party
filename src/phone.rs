use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use crate::silam::{Pollen, PollenIndex};

fn get_spoken_time(time: &DateTime<Utc>, tz: &Tz) -> String {
    time.with_timezone(tz).format("%-I%P").to_string()
}

fn is_pollen_very_low(pollen: &Pollen) -> bool {
    pollen.pollen_index == PollenIndex::VeryLow || pollen.pollen_index == PollenIndex::Unknown
}

pub fn get_phone_text(pollen_three_day: &Vec<Pollen>, now_index: usize, tz: Tz) -> String {
    let pollen_now = pollen_three_day.get(now_index).unwrap();
    let chunked_pollen = pollen_three_day.chunks(24).collect::<Vec<&[Pollen]>>();

    let [pollen_today, pollen_tomorrow, pollen_day_after] = <[&Pollen; 3]>::try_from(
        chunked_pollen
            .iter()
            .map(|chunk| {
                chunk
                    .iter()
                    .max_by(|a, b| a.pollen_index.cmp(&b.pollen_index))
                    .unwrap()
            })
            .collect::<Vec<&Pollen>>(),
    )
    .ok()
    .unwrap();

    let before_text = "Hello.";
    let now_text = format!(
        "Pollen at EMF is currently {}. The main source of pollen is {}.",
        pollen_now.pollen_index.to_spoken(),
        pollen_now.pollen_index_source.to_spoken()
    );
    let today_text = if pollen_today.time == pollen_now.time
        || pollen_today.pollen_index == pollen_now.pollen_index
    {
        format!("It is currently at today's high.")
    } else if is_pollen_very_low(pollen_today) {
        format!("Pollen will be very low all day.")
    } else if pollen_today.time < pollen_now.time {
        format!(
            "Today's high was {} {} at {}.",
            pollen_today.pollen_index.to_spoken(),
            pollen_today.pollen_index_source.to_spoken(),
            get_spoken_time(&pollen_today.time, &tz)
        )
    } else {
        format!(
            "Today's high will be {} {} at {}.",
            pollen_today.pollen_index.to_spoken(),
            pollen_today.pollen_index_source.to_spoken(),
            get_spoken_time(&pollen_today.time, &tz)
        )
    };
    let tomorrow_text = if is_pollen_very_low(pollen_tomorrow) {
        format!("Tomorrow pollen will be very low all day.")
    } else {
        format!(
            "Tomorrow's high will be {} {} at {}.",
            pollen_tomorrow.pollen_index.to_spoken(),
            pollen_tomorrow.pollen_index_source.to_spoken(),
            get_spoken_time(&pollen_tomorrow.time, &tz)
        )
    };
    let day_after_text = if is_pollen_very_low(pollen_day_after) {
        format!(
            "{}'s pollen will be very low all day.",
            pollen_day_after.time.format("%A")
        )
    } else {
        format!(
            "{}'s high will be {} {} at {}.",
            pollen_day_after.time.format("%A"),
            pollen_day_after.pollen_index.to_spoken(),
            pollen_day_after.pollen_index_source.to_spoken(),
            get_spoken_time(&pollen_day_after.time, &tz)
        )
    };
    let after_text = "Thank you for calling the EMF pollen hotline. Data provided by Finnish Meteorological Institute and European Aeroallergen Network. Goodbye.";
    let text = format!(
        "{} {} {} {} {} {}",
        before_text, now_text, today_text, tomorrow_text, day_after_text, after_text
    );
    text
}
