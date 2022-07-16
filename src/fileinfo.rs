#[derive(Clone, Debug)]
pub struct FileInfo {
    pub path: String,
    pub content: String,
    pub ext: String,
    pub name: String,
    pub is_folder: bool,
}
