use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct FileInfo {
    pub path: String,
    pub matches: Vec<Match>,
    pub ext: String,
    pub name: String,
    pub is_folder: bool,
}

impl FileInfo {
    pub fn content(&self, max_count: usize, max_length: usize) -> String {
        self.matches
            .iter()
            .take(max_count)
            .map(|x| {
                //limit content line length
                let fixed = match x.content.char_indices().nth(max_length) {
                    None => Cow::from(&x.content),
                    Some((idx, _)) => Cow::from(format!("{}...", &x.content[..idx])),
                };
                format!("{}: {}", x.line, fixed.trim_end())
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[derive(Clone, Debug)]
pub struct Match {
    pub line: usize,
    pub content: String,
}
