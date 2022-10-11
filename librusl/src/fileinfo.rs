#[derive(Clone, Debug)]
pub struct FileInfo {
    pub path: String,
    pub matches: Vec<Match>,
    pub ext: String,
    pub name: String,
    pub is_folder: bool,
}

impl FileInfo {
    pub fn content(&self) -> String {
        self.matches
            .iter()
            .map(|x| format!("{}: {}", x.line, x.content))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[derive(Clone, Debug)]
pub struct Match {
    pub line: usize,
    pub content: String,
}
