#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug)]
pub struct Node {
    pub id: u64,
    pub lon: f64,
    pub lat: f64,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug)]
pub struct Way {
    pub nodes: Vec<Node>,
}
