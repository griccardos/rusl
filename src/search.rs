#[derive(Clone)]
pub struct Search {
    pub dir: String,
    pub name_text: String,
    pub contents_text: String,
}
impl Default for Search {
    fn default() -> Self {
        Self {
            dir: ".".to_string(),
            name_text: "".to_string(),
            contents_text: "".to_string(),
        }
    }
}
