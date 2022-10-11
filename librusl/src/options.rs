use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Options {
    //general options
    #[serde(default)]
    pub sort: Sort,
    pub last_dir: String,
    pub name_history: Vec<String>,
    pub content_history: Vec<String>,
    pub name: NameOptions,
    pub content: ContentOptions,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            sort: Sort::None,
            last_dir: ".".to_string(),
            name_history: vec![],
            content_history: vec![],
            name: Default::default(),
            content: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NameOptions {
    pub case_sensitive: bool,
    pub file_types: FTypes,
    pub same_filesystem: bool,
    pub follow_links: bool,
    pub ignore_dot: bool,
}

impl Default for NameOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            file_types: FTypes::All,
            same_filesystem: false,
            follow_links: false,
            ignore_dot: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ContentOptions {
    pub case_sensitive: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Default)]
pub enum Sort {
    #[default]
    None,
    Path,
    Name,
    Extension,
}

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum FTypes {
    Files,
    Directories,
    All,
}
