use std::collections::HashMap;

use self::osm::{Node, OsmId, Relation, Way};

pub mod osm;

/// Map data as defined in the .osm file. Some elements are discarded but most are
/// kept without any processing.

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct OsmMapData {
    pub nodes: HashMap<OsmId, Node>,
    pub ways: HashMap<OsmId, Way>,
    pub relations: HashMap<OsmId, Relation>,
}

/// Collection of all semantic map elements (roads, tube stations, etc.) we could want to draw.

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct SemanticMapElements {

}
