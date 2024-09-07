mod etl;
mod data;
mod errors;

mod study;

use std::fs::{create_dir_all, File};
use std::io;
use std::path::{Path, PathBuf};
use std::str;

use etl::draw_map::DrawMapEtl;
use etl::semantic_map::SemanticMapEtl;
use serde::Deserialize;
use structured_logger::json::new_writer;
use structured_logger::Builder;
use raqote::Color;

use crate::etl::parse_osm::ParseOsmEtl;
use crate::etl::Etl;
use crate::errors::Result;

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
    #[serde(with = "serialize_color")]
    pub test_color: Color,
}

mod serialize_color {
    use std::collections::HashMap;

    use raqote::Color;
    use serde::{de, Deserialize, Deserializer, Serializer};
    use serde::de::{MapAccess, Visitor};


    struct ColorVisitor;

    impl<'de> Visitor<'de> for ColorVisitor {
        type Value = raqote::Color;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a JSON dictionary containg 'r', 'g', 'b', and 'a' keys")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where A: MapAccess<'de> {
            let mut validated_rgb_map: HashMap<String, u8> = HashMap::new();
            while let Some((key, val)) = map.next_entry::<String, u8>()? {
                if key == "r" || key == "g" || key == "b" || key == "a" {
                    if validated_rgb_map.contains_key(&key) {
                        return Err(de::Error::invalid_value(de::Unexpected::Str(&key), &self))
                    }
                    validated_rgb_map.insert(key, val);
                } else {
                    return Err(de::Error::invalid_value(de::Unexpected::Str(&key), &self))
                }
            }
            Ok(Color::new(
                validated_rgb_map["a"],
                validated_rgb_map["r"],
                validated_rgb_map["g"],
                validated_rgb_map["b"],
            ))
    }
}


    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<raqote::Color, D::Error>
        where D: Deserializer<'de> {
        deserializer.deserialize_map(ColorVisitor)
    }

    pub fn serialize<S>(
        color: &raqote::Color,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
        where S: Serializer {
        todo!("")
    }

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
    // study::run();
    // Ok(())

    setup_logging();

    //let user_config = load_user_config("../config/london_full.json");
    let user_config = load_user_config("../config/london_center.json");
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
