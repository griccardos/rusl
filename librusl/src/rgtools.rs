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
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use termcolor::NoColor;
use walkdir::WalkDir;

struct MyWrite {
    data: Vec<u8>,
}

pub const SEPARATOR: &str = r"\0\1\2\3\4";
pub const EXTENSION_SEPARATOR: &str = r"\5\6\7\8";

#[derive(Default)]
pub struct ContentResults {
    pub results: Vec<String>,
    pub errors: Vec<String>,
}
pub fn search_contents(
    pattern: &str,
    paths: &[OsString],
    allowed_files: &HashSet<String>,
    ops: ContentOptions,
    global_search_id: Arc<AtomicUsize>,
    start_search_id: usize,
    total_search_count: Option<Arc<AtomicUsize>>, //only Some if find contents, else None because we would have counted the file in the name search
) -> ContentResults {
    let case_insensitive = !ops.case_sensitive;
    let mut errors = vec![];
    //TODO: there is a fixed_strings in RegexMatcherBuilder, but it is not updated on
    //crates.io yet, so we manually escape
    let mut pattern = pattern.to_string();
    if ops.nonregex {
        pattern = regex::escape(&pattern);
    }
    ////

    let matcher = RegexMatcherBuilder::new().case_insensitive(case_insensitive).build(&pattern);

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

    if allowed_files.len() > 0 {
        for path in allowed_files {
            if global_search_id.load(Ordering::Relaxed) != start_search_id {
                return ContentResults::default();
            }
            read_file(
                &Path::new(&path),
                &mut searcher,
                &matcher,
                &mut printer,
                &mut errors,
                &ops,
                total_search_count.clone(),
            );
        }
    } else {
        for path in paths {
            for result in WalkDir::new(path) {
                if global_search_id.load(Ordering::Relaxed) != start_search_id {
                    return ContentResults::default();
                }

                let dent = match result {
                    Ok(dent) => dent,
                    Err(_err) => {
                        errors.push(format!("Could not read file {path:?}"));
                        continue;
                    }
                };

                if !dent.file_type().is_file() {
                    continue;
                }
                read_file(
                    &dent.path(),
                    &mut searcher,
                    &matcher,
                    &mut printer,
                    &mut errors,
                    &ops,
                    total_search_count.clone(),
                );
            }
        }
    }
    let strings: Vec<String> = printer.into_inner().into_inner().string().split('\n').map(|x| x.to_string()).collect();

    ContentResults { results: strings, errors }
}
fn read_file(
    path: &Path,
    searcher: &mut Searcher,
    matcher: &RegexMatcher,
    printer: &mut Standard<NoColor<MyWrite>>,
    errors: &mut Vec<String>,
    ops: &ContentOptions,
    total_search_count: Option<Arc<AtomicUsize>>,
) {
    if let Some(total_search_count) = total_search_count.as_ref() {
        total_search_count.fetch_add(1, Ordering::Relaxed);
    }
    let file = File::open(path);
    match file {
        Ok(file) => {
            //normal grep
            let result = searcher.search_file(matcher, &file, printer.sink_with_path(matcher, &path));
            if let Err(_err) = result {
                errors.push(format!("Could not read file {path:?}"));
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
                            printer.sink_with_path(matcher, &format!("{}{}{}", path.to_string_lossy(), EXTENSION_SEPARATOR, ext.name())),
                        );
                        if let Err(_err) = result {
                            errors.push(format!("Could not read file {path:?} with extension {ext:?}"));
                        }
                    } else {
                    }
                }
            }
        }
        Err(_err) => {
            errors.push(format!("Could not read {}", path.as_os_str().to_string_lossy()));
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
