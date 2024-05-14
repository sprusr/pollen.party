use chrono::{DateTime, Duration, NaiveTime, SecondsFormat, Utc};
use ndarray::{s, Array3, Ix3};
use proj4rs::Proj;
use serde::Serialize;
use std::{cmp::max, fmt::Display};

#[derive(Debug, Serialize, Clone, Copy)]
pub enum PollenIndex {
    Unknown,
    VeryLow,
    Low,
    Moderate,
    High,
    VeryHigh,
}

impl PollenIndex {
    pub fn from_raw(raw: &f32) -> PollenIndex {
        match *raw as i32 {
            1 => PollenIndex::VeryLow,
            2 => PollenIndex::Low,
            3 => PollenIndex::Moderate,
            4 => PollenIndex::High,
            5 => PollenIndex::VeryHigh,
            _ => PollenIndex::Unknown,
        }
    }

    pub fn to_spoken(&self) -> &str {
        match self {
            PollenIndex::Unknown => "unknown",
            PollenIndex::VeryLow => "very low",
            PollenIndex::Low => "low",
            PollenIndex::Moderate => "moderate",
            PollenIndex::High => "high",
            PollenIndex::VeryHigh => "very high",
        }
    }
}

impl Display for PollenIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PollenIndex::Unknown => write!(f, "0"),
            PollenIndex::VeryLow => write!(f, "1"),
            PollenIndex::Low => write!(f, "2"),
            PollenIndex::Moderate => write!(f, "3"),
            PollenIndex::High => write!(f, "4"),
            PollenIndex::VeryHigh => write!(f, "5"),
        }
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
pub enum PollenType {
    Unknown = -1,
    Alder = 1,
    Birch = 2,
    Grass = 3,
    Olive = 4,
    Mugwort = 5,
    Ragweed = 6,
}

impl PollenType {
    pub fn from_raw(raw: &f32) -> PollenType {
        match *raw as i32 {
            -1 => PollenType::Unknown,
            1 => PollenType::Alder,
            2 => PollenType::Birch,
            3 => PollenType::Grass,
            4 => PollenType::Olive,
            5 => PollenType::Mugwort,
            6 => PollenType::Ragweed,
            _ => PollenType::Unknown,
        }
    }

    pub fn to_spoken(&self) -> &str {
        match self {
            PollenType::Unknown => "unknown",
            PollenType::Alder => "alder",
            PollenType::Birch => "birch",
            PollenType::Grass => "grass",
            PollenType::Olive => "olive",
            PollenType::Mugwort => "mugwort",
            PollenType::Ragweed => "ragweed",
        }
    }
}

impl Display for PollenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PollenType::Unknown => write!(f, "???"),
            PollenType::Alder => write!(f, "Alder"),
            PollenType::Birch => write!(f, "Birch"),
            PollenType::Grass => write!(f, "Grass"),
            PollenType::Olive => write!(f, "Olive"),
            PollenType::Mugwort => write!(f, "Mugwort"),
            PollenType::Ragweed => write!(f, "Ragweed"),
        }
    }
}

#[derive(Serialize, Clone, Copy)]
pub struct Pollen {
    pub time: DateTime<Utc>,
    pub pollen_index: PollenIndex,
    pub pollen_index_source: PollenType,
}

pub struct Silam {
    pub fetch_time: DateTime<Utc>,
    pub start_time: DateTime<Utc>,
    poli: Array3<f32>,
    polisrc: Array3<f32>,
    rlats: Vec<f32>,
    rlons: Vec<f32>,
}

impl Silam {
    pub async fn fetch(silam_email: &Option<String>) -> Result<Silam, Box<dyn std::error::Error>> {
        let start_time = Utc::now()
            .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap()
            - Duration::days(1);
        let end_time = start_time + Duration::hours(23) + Duration::days(4);
        let silam_email_param = match silam_email {
            Some(email) => format!("&email={}", email),
            None => String::new(),
        };
        let silam_url = format!(
            "https://thredds.silam.fmi.fi/thredds/ncss/grid/silam_europe_pollen_v5_9/silam_europe_pollen_v5_9_best.ncd?var=POLI&var=POLISRC&north=75.950&west=-47.600&east=78.059&south=19.003&horizStride=1&accept=netcdf4ext&addLatLon=true&time_start={}&time_end={}{}",
            start_time.to_rfc3339_opts(SecondsFormat::Secs, true),
            end_time.to_rfc3339_opts(SecondsFormat::Secs, true),
            silam_email_param,
        );
        println!("Fetching new data from SILAM: {}", silam_url);
        let body = reqwest::get(silam_url).await?.bytes().await?;
        let file = netcdf::open_mem(None, &body)?;

        let rlons: Vec<f32> = file
            .variable("rlon")
            .expect("rlon variable missing")
            .get_values(..)
            .expect("rlon could not be parsed");
        let rlats: Vec<f32> = file
            .variable("rlat")
            .expect("rlat variable missing")
            .get_values(..)
            .expect("rlat could not be parsed");
        let poli: Array3<f32> = file
            .variable("POLI")
            .expect("POLI variable missing")
            .get::<f32, _>(..)?
            .into_dimensionality::<Ix3>()
            .expect("POLI could not be parsed as Array3");
        let polisrc: Array3<f32> = file
            .variable("POLISRC")
            .expect("POLISRC variable missing")
            .get::<f32, _>(..)?
            .into_dimensionality::<Ix3>()
            .expect("POLISRC could not be parsed as Array3");

        Ok(Silam {
            fetch_time: Utc::now(),
            start_time,
            poli,
            polisrc,
            rlats,
            rlons,
        })
    }

    fn stale_at(&self) -> DateTime<Utc> {
        self.fetch_time + Duration::hours(12)
    }

    pub fn time_until_stale(&self) -> Duration {
        max(self.stale_at() - Utc::now(), Duration::zero())
    }

    pub fn is_stale(&self) -> bool {
        self.time_until_stale() == Duration::zero()
    }

    pub fn get_at_coords(&self, lon: &f32, lat: &f32) -> Vec<Pollen> {
        let (projected_lon, projected_lat) = project_lon_lat(lon, lat);

        let closest_rlon_index = find_closest(&self.rlons, projected_lon).unwrap();
        let closest_rlat_index = find_closest(&self.rlats, projected_lat).unwrap();

        let pollen_indexes = self
            .poli
            .slice(s![.., closest_rlat_index, closest_rlon_index]); // apparently index here works by lat/lon, not lon/lat!

        pollen_indexes
            .iter()
            .enumerate()
            .map(|(i, pollen_index)| Pollen {
                pollen_index: PollenIndex::from_raw(pollen_index),
                pollen_index_source: PollenType::from_raw(
                    self.polisrc
                        .get((i, closest_rlat_index, closest_rlon_index))
                        .unwrap(),
                ),
                time: self.start_time + Duration::hours(i.try_into().unwrap()),
            })
            .collect()
    }
}

fn project_lon_lat(lon: &f32, lat: &f32) -> (f32, f32) {
    let lonlat = Proj::from_proj_string("+proj=longlat").unwrap();
    let tmerc = Proj::from_proj_string("+proj=tmerc +lon_0=0 +lat_0=0").unwrap();
    let rotated = Proj::from_proj_string("+proj=tmerc +lon_0=0 +lat_0=-60").unwrap();

    let mut point_3d = (lon.to_radians() as f64, lat.to_radians() as f64, 0.0);
    proj4rs::transform::transform(&lonlat, &tmerc, &mut point_3d).unwrap();
    proj4rs::transform::transform(&rotated, &lonlat, &mut point_3d).unwrap();

    (
        point_3d.0.to_degrees() as f32,
        point_3d.1.to_degrees() as f32,
    )
}

fn find_closest(vec: &Vec<f32>, target: f32) -> Option<usize> {
    match vec.binary_search_by(|probe| probe.partial_cmp(&target).unwrap()) {
        Ok(index) => Some(index), // Exact match found
        Err(index) => {
            if index == 0 {
                Some(0) // Target is less than all elements, so closest is the first
            } else if index == vec.len() {
                Some(vec.len() - 1) // Target is greater than all elements, so closest is the last
            } else {
                // Check which of the neighbors is closer to the target
                let prev_diff = target - vec[index - 1];
                let next_diff = vec[index] - target;
                if prev_diff > next_diff {
                    Some(index)
                } else {
                    Some(index - 1)
                }
            }
        }
    }
}
