use std::fmt::Display;

use ndarray::{Array3, Ix3};
use proj::Proj;

#[derive(Debug)]
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
}

impl Display for PollenIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
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
}

impl Display for PollenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Pollen {
    pub pollen_index: PollenIndex,
    pub pollen_index_source: PollenType,
}

pub struct Silam {
    poli: Array3<f32>,
    polisrc: Array3<f32>,
    rlats: Vec<f32>,
    rlons: Vec<f32>,
}

// &time_start=2024-03-31T01:00:00Z&time_end=2024-04-08T00:00:00Z
const SILAM_URL: &str = "https://thredds.silam.fmi.fi/thredds/ncss/grid/silam_europe_pollen_v5_9/silam_europe_pollen_v5_9_best.ncd?var=POLI&var=POLISRC&north=75.950&west=-47.600&east=78.059&south=19.003&horizStride=1&accept=netcdf4ext&addLatLon=true";

impl Silam {
    pub async fn fetch() -> Result<Silam, Box<dyn std::error::Error>> {
        let body = reqwest::get(SILAM_URL).await?.bytes().await?;
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
            poli,
            polisrc,
            rlats,
            rlons,
        })
    }

    pub fn get_first_at_coords(&self, lon: &f32, lat: &f32) -> Pollen {
        let (projected_lon, projected_lat) = project_lon_lat(lon, lat);

        let closest_rlon_index = find_closest(&self.rlons, projected_lon).unwrap();
        let closest_rlat_index = find_closest(&self.rlats, projected_lat).unwrap();

        let pollen_index = self
            .poli
            .get((0, closest_rlat_index, closest_rlon_index))
            .unwrap();

        let pollen_index_source = self
            .polisrc
            .get((0, closest_rlat_index, closest_rlon_index))
            .unwrap();

        Pollen {
            pollen_index: PollenIndex::from_raw(pollen_index),
            pollen_index_source: PollenType::from_raw(pollen_index_source),
        }
    }
}

const PROJECTION_FROM: &str = "+proj=longlat";
const PROJECTION_TO: &str = "+proj=ob_tran +o_proj=longlat +o_lon_p=0 +o_lat_p=30";

fn project_lon_lat(lon: &f32, lat: &f32) -> (f32, f32) {
    // TODO: error handling
    let projection = Proj::new_known_crs(PROJECTION_FROM, PROJECTION_TO, None).unwrap();
    projection.convert((*lon, *lat)).unwrap()
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