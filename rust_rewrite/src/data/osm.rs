use std::collections::HashMap;

pub type OsmId = u64;

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
    pub nodes: Vec<Node>,
    pub tags: HashMap<Vec<u8>, Vec<u8>>,
}
