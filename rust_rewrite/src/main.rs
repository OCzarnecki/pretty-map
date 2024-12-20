mod etl;
mod data;
mod errors;

mod study;

use std::fs::{create_dir_all, File};
use std::io;
use std::path::{Path, PathBuf};
use std::str;

use etl::draw_map::{Theme, DrawMapEtl};
use etl::semantic_map::SemanticMapEtl;
use serde::Deserialize;
use structured_logger::json::new_writer;
use structured_logger::Builder;

use crate::etl::parse_osm::ParseOsmEtl;
use crate::etl::Etl;
use crate::errors::Result;

#[derive(Deserialize)]
pub struct UserConfig<'a> {
    pub data_path: String,
    pub dest_path: String,
    pub top_left_lon: f64,
    pub top_left_lat: f64,
    pub px_per_deg_lon: f64,
    pub px_per_deg_lat: f64,
    pub width_px: u64,
    pub height_px: u64,
    pub theme: Theme<'a>,
}


fn load_user_config(path: &str) -> UserConfig {
    let file = File::open(path).expect("Could not open config file.");
    serde_json::from_reader(file).expect("Could not parse config.")
}


fn create_output_dir(config: &UserConfig) -> Result<PathBuf> {
    let input_fname = Path::new(&config.data_path)
        .file_name()
        .ok_or("Could not get input file name")?;
    let output_dir = Path::new("output").join(input_fname);
    create_dir_all(&output_dir)?;
    Ok(output_dir)
}

fn setup_logging() {
    Builder::with_level("info")
        .with_target_writer("*", new_writer(io::stdout()))
        .init();
}

fn main() -> Result<()> {
    //crate::study::big_image();
    //return Ok(());

    setup_logging();

    let user_config = load_user_config("../config/london_full.json");
    // let user_config = load_user_config("../config/london_center.json");
    let output_dir = create_output_dir(&user_config)?;

    // Limit ETL Scope so that memory can be freed as early as possible
    {
        let mut parse_osm_etl = ParseOsmEtl::new(&user_config);
        parse_osm_etl.process(&output_dir)?;
    }
    {
        let mut semantic_map_etl = SemanticMapEtl::new();
        semantic_map_etl.process(&output_dir)?;
    }
    {
        let mut draw_map_etl = DrawMapEtl::new(&user_config);
        draw_map_etl.process(&output_dir)?;
    }

    Ok(())
}
