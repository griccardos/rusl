use crate::{
    extended::{ExtendedTrait, ExtendedType},
    options::ContentOptions,
};
use grep::{
    printer::{Standard, StandardBuilder},
    regex::{RegexMatcher, RegexMatcherBuilder},
    searcher::{BinaryDetection, Searcher, SearcherBuilder},
};
use std::{
    collections::HashSet,
    ffi::OsString,
    fs::File,
    io::{Cursor, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
use termcolor::NoColor;
use walkdir::WalkDir;

struct MyWrite {
    data: Vec<u8>,
}

pub const SEPARATOR: &str = r"\0\1\2\3\4";

#[derive(Default)]
pub struct ContentResults {
    pub results: Vec<String>,
    pub errors: Vec<String>,
}
pub fn search_contents(
    pattern: &str,
    paths: &[OsString],
    allowed_files: Option<HashSet<String>>,
    ops: ContentOptions,
    must_stop: Arc<AtomicBool>,
    counter: Arc<AtomicUsize>,
) -> ContentResults {
    let case_insensitive = !ops.case_sensitive;
    let mut errors = vec![];
    let matcher = RegexMatcherBuilder::new().case_insensitive(case_insensitive).build(pattern);
    if matcher.is_err() {
        return ContentResults::default();
    }
    let matcher = matcher.unwrap();

    let mut searcher = SearcherBuilder::new()
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .line_number(true)
        .build();

    let my_write = MyWrite { data: vec![] };
    let mut printer = StandardBuilder::new()
        .separator_field_match(SEPARATOR.as_bytes().to_vec())
        .build_no_color(my_write);

    if let Some(allowed_files) = allowed_files {
        for path in allowed_files {
            read_file(
                &Path::new(&path),
                &mut searcher,
                &matcher,
                &mut printer,
                &mut errors,
                &ops,
                counter.clone(),
            );
        }
    } else {
        for path in paths {
            for result in WalkDir::new(path) {
                if must_stop.load(Ordering::Relaxed) {
                    return ContentResults::default();
                }

                let dent = match result {
                    Ok(dent) => dent,
                    Err(err) => {
                        errors.push(err.to_string());
                        continue;
                    }
                };

                if !dent.file_type().is_file() {
                    continue;
                }
                read_file(&dent.path(), &mut searcher, &matcher, &mut printer, &mut errors, &ops, counter.clone());
            }
        }
    }
    ContentResults {
        results: printer.into_inner().into_inner().string().split('\n').map(|x| x.to_string()).collect(),
        errors,
    }
}
fn read_file(
    path: &Path,
    searcher: &mut Searcher,
    matcher: &RegexMatcher,
    printer: &mut Standard<NoColor<MyWrite>>,
    errors: &mut Vec<String>,
    ops: &ContentOptions,
    counter: Arc<AtomicUsize>,
) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |f| Some(f + 1));

    let file = File::open(path);
    if let Ok(file) = file {
        //normal grep
        let result = searcher.search_file(matcher, &file, printer.sink_with_path(matcher, &path));
        if let Err(err) = result {
            errors.push(err.to_string());
        }

        //apply each of extensions
        if ops.extended {
            let extension = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
            let extendeds = vec![ExtendedType::Pdf, ExtendedType::Office];
            for ext in extendeds.iter().filter(|a| a.extensions().contains(&extension)) {
                if let Ok(data) = ext.to_string(&path) {
                    let cursor = Cursor::new(data);
                    let result = searcher.search_reader(
                        matcher,
                        cursor,
                        printer.sink_with_path(matcher, &format!("{} ({})", path.to_string_lossy(), ext.name())),
                    );
                    if let Err(err) = result {
                        errors.push(err.to_string());
                    }
                } else {
                }
            }
        }
    }
}
impl MyWrite {
    pub fn string(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }
}

impl Write for MyWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
