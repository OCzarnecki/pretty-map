use std::{str, collections::HashMap, fs::{self, File}, io::{Read, Write}, path::{Path, PathBuf}};

use crate::{
    data::{
        osm::{Node, OsmId, OsmMapData, Relation, Way},
        semantic::{
            Area,
            AreaType,
            Landmark,
            LandmarkType,
            SemanticMapElements,
            TransportStation,
            TransportStationType,
            TubeLine,
            TubeRail,
        }
    },
    errors::Result,
};
use crate::etl::parse_osm;

use super::Etl;
use quick_xml::escape::unescape;
use regex::Regex;

pub const ETL_NAME: &str = "semantic_map";
pub const OUTPUT_FILE_NAME: &str = "semantic_map.rkyv";

pub struct SemanticMapEtl {
}

impl SemanticMapEtl {
    fn output_path(dir: &Path) -> PathBuf {
        dir.join(OUTPUT_FILE_NAME)
    }

    fn has_key(tags: &HashMap<Vec<u8>, Vec<u8>>, key: &[u8]) -> bool {
        let key_vec = key.to_vec();
        tags.contains_key(&key_vec)
    }

    fn has_kv_pair(tags: &HashMap<Vec<u8>, Vec<u8>>, key: &[u8], value: &[u8]) -> bool {
        let key_vec = key.to_vec();
        if let Some(tag_value) = tags.get(&key_vec) {
            tag_value.split(|b| *b == 59)
                .any(|tag| tag == value)
        } else {
            false
        }
    }

    fn get_string(tags: &HashMap<Vec<u8>, Vec<u8>>, key: &[u8]) -> Option<String> {
        let key_vec = key.to_vec();
        let val_vec = tags.get(&key_vec)?;
        Some(
            unescape(
                str::from_utf8(val_vec).ok()?
            ).ok()?.to_string()
        )
    }

    fn get_strings(tags: &HashMap<Vec<u8>, Vec<u8>>, key: &[u8]) -> Vec<String> {
        Self::get_string(tags, key)
            .map(|s| s.split(";").map(|s| s.to_string()).collect::<Vec<String>>())
            .into_iter()
            .flatten()
            .collect()
    }

    fn process_nodes(&mut self, output: &mut SemanticMapElements, nodes: &HashMap<OsmId, Node>) {
        for node in nodes.values() {
            if Self::has_kv_pair(&node.tags, b"railway", b"station")
                // && Self::has_kv_pair(&node.tags, b"subway", b"yes")
                && Self::has_key(&node.tags, b"name") {

                let name = Self::get_string(&node.tags, b"name").unwrap();

                // Some names are like "Edgeware Road (Bakerloo line)", we want to strip the
                // brackets.
                let re = Regex::new(r"(?<base_name>[^(]*)(\(.*\))?").unwrap();
                let base_name = re.captures(&name).unwrap().name("base_name").unwrap().as_str();
                let maybe_station_type = if Self::has_kv_pair(&node.tags, b"network", b"London Underground") {
                    Some(TransportStationType::Underground)
                } else if Self::has_kv_pair(&node.tags, b"network", b"Docklands Light Railway"){
                    Some(TransportStationType::Dlr)
                } else if Self::has_kv_pair(&node.tags, b"network", b"London Overground") {
                    Some(TransportStationType::Overground)
                } else if Self::has_kv_pair(&node.tags, b"network", b"Elizabeth Line") {
                    Some(TransportStationType::ElizabethLine)
                } else {
                    None
                };
                if let Some(station_type) = maybe_station_type {
                    output.underground_stations.push(
                        TransportStation {
                            name: base_name.trim().to_string(),
                            station_type,
                            lon: node.lon,
                            lat: node.lat
                        }
                    );
                }
            }
            let maybe_landmark_type = if Self::has_kv_pair(&node.tags, b"lgbtq:men", b"only")
                || Self::has_kv_pair(&node.tags, b"lgbtq:men", b"primary")
                || Self::has_kv_pair(&node.tags, b"gay", b"yes") {
                    Some(LandmarkType::LgbtqMen)
            } else if Self::has_kv_pair(&node.tags, b"lgbtq", b"primary") {
                Some(LandmarkType::Lgbtq)
            } else if Self::has_kv_pair(&node.tags, b"bar", b"cocktail")
                || Self::has_kv_pair(&node.tags, b"cocktails", b"yes")
                || Self::has_kv_pair(&node.tags, b"drink:cocktail", b"served") {
                Some(LandmarkType::CocktailBar)
            } else if Self::has_kv_pair(&node.tags, b"emergency", b"emergency_ward_entrance")
                || Self::has_kv_pair(&node.tags, b"healthcare", b"emergency_ward") {
                Some(LandmarkType::Hospital)
            } else if Self::has_kv_pair(&node.tags, b"natural", b"tree") {
                Some(LandmarkType::Tree)
            } else {
                None
            };
            if let Some(landmark_type) = maybe_landmark_type {
                output.landmarks.push(
                    Landmark{
                        lon: node.lon,
                        lat: node.lat,
                        landmark_type,
                    }
                );
            }
        }
    }

    fn process_ways(&mut self, output: &mut SemanticMapElements, ways: &HashMap<OsmId, Way>) {
        for way in ways.values() {
            if Self::has_key(&way.tags, b"highway") {
                output.roads.push(way.into());
            }
            if Self::has_kv_pair(&way.tags, b"leisure", b"park") {
                output.areas.push(
                    Area::new(
                        AreaType::Park,
                        &vec![way.into()],
                    )
                );
            }
            if Self::has_key(&way.tags, b"water") 
                || Self::has_kv_pair(&way.tags, b"natural", b"water")
            {
                output.areas.push(
                    Area::new(
                        AreaType::Water,
                        &vec![way.into()],
                    )
                );
            }
            if (
                Self::has_kv_pair(&way.tags, b"railway", b"subway")
                || Self::has_kv_pair(&way.tags, b"railway", b"rail")
            ) && Self::has_key(&way.tags, b"line") {
                for line_tag in Self::get_strings(&way.tags, b"line") {
                    let lines = match line_tag.to_lowercase().as_str() {
                        "bakerloo" => vec![TubeLine::Bakerloo],
                        "central" | "central line" => vec![TubeLine::Central],
                        "circle" => vec![TubeLine::Circle],
                        "deep level district" | "district" | "district, north london" => vec![TubeLine::District],
                        "district, piccadilly" => vec![TubeLine::District, TubeLine::Piccadilly],
                        "dlr" => vec![TubeLine::Dlr],
                        "elizabeth" => vec![TubeLine::Elizabeth],
                        "hammersmith & city" => vec![TubeLine::HammersmithAndCity],
                        "jubilee" | "jubilee line" => vec![TubeLine::Jubilee],
                        "metropolitan" => vec![TubeLine::Metropolitan],
                        "metropolitan, piccadilly" => vec![TubeLine::Metropolitan, TubeLine::Piccadilly],
                        "northern" | "northern line" => vec![TubeLine::Northern],
                        "northern city" => vec![TubeLine::Northern, TubeLine::HammersmithAndCity],
                        "picadilly" | "piccadilly" => vec![TubeLine::Piccadilly],
                        "victoria" => vec![TubeLine::Victoria],
                        "waterloo & city" => vec![TubeLine::WaterlooAndCity],
                        _ => {
                            vec![]
                        },
                    };
                    for line in lines {
                        output.tube_rails.push(TubeRail {
                            line,
                            path: way.into(),
                        });
                    }
                }
            } else if Self::has_kv_pair(&way.tags, b"railway", b"rail") {
                output.rails.push(way.into());
            }
        }
    }

    fn process_relations(&mut self, output: &mut SemanticMapElements, relations: &HashMap<OsmId, Relation>) {
        for relation in relations.values() {
            if Self::has_kv_pair(&relation.tags, b"leisure", b"park") {
                output.areas.push(
                    Area::new(
                        AreaType::Park,
                        &relation.ways.iter().map(|way| way.into()).collect(),
                    )
                );
            }
            if Self::has_key(&relation.tags, b"water")
                || Self::has_kv_pair(&relation.tags, b"natural", b"water")
            {
                output.areas.push(
                    Area::new(
                        AreaType::Water,
                        &relation.ways.iter().map(|way| way.into()).collect(),
                    )
                );
            }
        }
    }

    pub fn new() -> SemanticMapEtl {
        SemanticMapEtl {}
    }
}

impl Etl for SemanticMapEtl {
    type Input = OsmMapData;
    type Output = SemanticMapElements;

    fn etl_name(&self) -> &str {
        ETL_NAME
    }

    fn is_cached(&self, dir: &std::path::Path) -> Result<bool> {
        Ok(Self::output_path(dir).exists())
    }

    fn clean(&self, dir: &std::path::Path) -> Result<()> {
        fs::remove_file(Self::output_path(dir))?;
        Ok(())
    }

    fn extract(&mut self, dir: &std::path::Path) -> Result<Self::Input> {
        let input_file_path = dir.join(parse_osm::OUTPUT_FILE_NAME);
        let mut input_file = File::open(input_file_path)?;

        let mut buf_vec: Vec<u8> = Vec::new();
        input_file.read_to_end(&mut buf_vec).expect("Could not read note cache.");

        let input: OsmMapData = unsafe {
            rkyv::from_bytes_unchecked(&buf_vec).expect("Could not deserialize node cache.")
        };

        Ok(input)
    }

    fn transform(&mut self, input: Self::Input) -> Result<Self::Output> {
        let mut output = SemanticMapElements::default();

        self.process_nodes(&mut output, &input.nodes);
        self.process_ways(&mut output, &input.ways);
        self.process_relations(&mut output, &input.relations);

        Ok(output)
    }

    fn load(&mut self, dir: &std::path::Path, output: Self::Output) -> Result<()> {
        let mut output_file = File::create(Self::output_path(dir))?;
        let bytes = rkyv::to_bytes::<_, 256>(&output).unwrap();
        output_file.write_all(&bytes)?;
        Ok(())
    }

}
