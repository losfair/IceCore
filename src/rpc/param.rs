#[derive(Serialize, Deserialize, Clone)]
pub enum Param {
    Integer(i32),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Error(Box<Param>),
    Array(Vec<Param>)
}
