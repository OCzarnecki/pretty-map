use std::{collections::HashMap, fs::{self, File}, io::{Read, Write}, path::{Path, PathBuf}};

use crate::{data::{osm::{Node, OsmId, OsmMapData, Relation, Way}, semantic::SemanticMapElements}, errors::Result};
use crate::etl::parse_osm;

use super::Etl;

const ETL_NAME: &str = "semantic_map";
const OUTPUT_FILE_NAME: &str = "semantic_map.rkyv";

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
        let val_vec = value.to_vec();
        if let Some(tag_value) = tags.get(&key_vec) {
            tag_value == &val_vec
        } else {
            false
        }
    }

    fn process_nodes(&mut self, output: &mut SemanticMapElements, nodes: &HashMap<OsmId, Node>) {
        for node in nodes.values() {
            if Self::has_kv_pair(&node.tags, b"railway", b"stop")
                && Self::has_kv_pair(&node.tags, b"subway", b"yes")
                && Self::has_kv_pair(&node.tags, b"public_transport", b"stop_position") {
                output.underground_stations.push(node.into());
            }
        }
    }

    fn process_ways(&mut self, output: &mut SemanticMapElements, ways: &HashMap<OsmId, Way>) {
        for way in ways.values() {
            if Self::has_kv_pair(&way.tags, b"railway", b"rail") {
                output.rails.push(way.into());
            }
            if Self::has_key(&way.tags, b"highway") {
                output.roads.push(way.into());
            }
            if Self::has_kv_pair(&way.tags, b"waterway", b"river")
                || Self::has_kv_pair(&way.tags, b"waterway", b"canal")
                || Self::has_kv_pair(&way.tags, b"waterway", b"ditch")
                || Self::has_kv_pair(&way.tags, b"waterway", b"drain") {
                output.narrow_waterways.push(way.into());
            }
        }
    }

    fn process_relations(&mut self, output: &mut SemanticMapElements, relations: &HashMap<OsmId, Relation>) {
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
