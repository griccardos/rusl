use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct Options {
    //general options
    pub sort: Sort,
    pub last_dir: String,
    pub name_history: Vec<String>,
    pub content_history: Vec<String>,
    pub name: NameOptions,
    pub content: ContentOptions,
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

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum Sort {
    Path,
    Name,
    Extension,
}
impl Default for Sort {
    fn default() -> Self {
        Sort::Path
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum FTypes {
    Files,
    Directories,
    All,
}
