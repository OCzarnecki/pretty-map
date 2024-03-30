mod etl;
mod data;
mod errors;

use std::fs::{create_dir_all, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::str;


use serde::Deserialize;
use structured_logger::json::new_writer;
use structured_logger::Builder;

use crate::etl::parse_osm::ParseOSMETL;
use crate::etl::ETL;
use crate::errors::Result;
use crate::data::osm::Node;

#[derive(Deserialize)]
pub struct UserConfig {
    pub data_path: String,
    pub dest_path: String,
    pub top_left_lon: f64,
    pub top_left_lat: f64,
    pub px_per_deg_lon: f64,
    pub px_per_deg_lat: f64,
    pub width_px: u64,
    pub height_px: u64,
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
    setup_logging();


    let user_config = load_user_config("../config/london_full.json");
    let etl = ParseOSMETL::new(&user_config);
    let output_dir = create_output_dir(&user_config)?;
    etl.process(Path::new(&output_dir))?;

    let output_path = output_dir.join("osm_elements.rkyv");
    let mut fin = File::open(output_path).expect("Could not open node cache file.");
    let mut buf_vec: Vec<u8> = Vec::new();
    fin.read_to_end(&mut buf_vec).expect("Could not read note cache.");
    let nodes: Vec<Node> = unsafe {
        rkyv::from_bytes_unchecked(&buf_vec).expect("Could not deserialize node cache.")
    };
    eprintln!("Read {} nodes from cache.", nodes.len());

    Ok(())
}
