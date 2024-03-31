use super::osm::{Node, Way};

/// Collection of all semantic map elements (roads, tube stations, etc.) we could want to draw.

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct SemanticMapElements {
    pub underground_stations: Vec<MapCoord>,
    pub rails: Vec<Path>,
    pub roads: Vec<Path>,
    pub narrow_waterways: Vec<Path>,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub struct MapCoord {
    pub lat: f64,
    pub lon: f64,
}

impl From<&Node> for MapCoord {
    fn from(value: &Node) -> Self {
        MapCoord {
            lat: value.lat,
            lon: value.lon,
        }
    }
}

type Path = Vec<MapCoord>;

impl From<&Way> for Path {
    fn from(value: &Way) -> Self {
        value.nodes.iter()
            .map(|el| el.into())
            .collect()
    }
}
