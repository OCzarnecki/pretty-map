use std::collections::HashMap;

pub type OsmId = u64;

/// Map data as defined in the .osm file. Some elements are discarded but most are
/// kept without any processing.

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct OsmMapData {
    pub nodes: HashMap<OsmId, Node>,
    pub ways: HashMap<OsmId, Way>,
    pub relations: HashMap<OsmId, Relation>,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct Node {
    pub id: OsmId,
    pub lon: f64,
    pub lat: f64,
    pub tags: HashMap<Vec<u8>, Vec<u8>>,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct Way {
    pub id: OsmId,
    pub nodes: Vec<Node>,
    pub tags: HashMap<Vec<u8>, Vec<u8>>,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct Relation {
    pub id: OsmId,
    pub ways: Vec<Way>,
    pub tags: HashMap<Vec<u8>, Vec<u8>>,
}
