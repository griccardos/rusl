use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use clipboard::{ClipboardContext, ClipboardProvider};
use ignore::WalkBuilder;

use crate::extended::ExtendedType;
use crate::fileinfo::{FileInfo, Match};
use crate::options::{FTypes, Options, Sort};
use crate::rgtools::{self, EXTENSION_SEPARATOR, SEPARATOR};
use crate::search::Search;

pub enum Message {
    File(FileInfo, usize),
    Done(usize, Duration),
    ContentFiles(Vec<FileInfo>, usize, Duration),
    StartSearch(usize),
    FileErrors(Vec<String>),
    Quit,
}

#[derive(Debug)]
pub enum SearchResult {
    FinalResults(FinalResults),
    InterimResult(FileInfo),
    SearchErrors(Vec<String>),
}
#[derive(Debug, Clone)]
pub struct FinalResults {
    pub data: Vec<FileInfo>,
    pub duration: Duration,
    pub id: usize,
}
pub struct Manager {
    internal_sender: Sender<Message>,
    id: usize,
    options: Arc<Mutex<Options>>,
    pub must_stop: Arc<AtomicBool>,
    pub counter: Arc<AtomicUsize>,
}

impl Manager {
    pub fn new(external_sender: Sender<SearchResult>) -> Self {
        let ops = load_options();
        let ops = Arc::new(Mutex::new(ops));

        //internal channel that sends results inside
        let (s, r) = std::sync::mpsc::channel();
        let ops_for_receiver = ops.clone();
        thread::spawn(move || {
            message_receiver(r, external_sender, ops_for_receiver);
        });
        Self {
            internal_sender: s,
            id: 0,
            options: ops,
            must_stop: Arc::new(AtomicBool::new(false)),
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }
    pub fn stop(&mut self) {
        self.must_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn get_count(&self) -> usize {
        self.counter.load(Ordering::Relaxed)
    }

    pub fn search(&mut self, search: Search) {
        //start with must stop=TRUE in case old one is running
        self.must_stop.store(false, Ordering::Relaxed);
        let mut ops = self.options.lock().unwrap();
        self.id += 1;
        self.counter.store(0, Ordering::Relaxed);
        ops.last_dir = search.dir.clone();
        if !search.name_text.is_empty() && !ops.name_history.contains(&search.name_text) {
            ops.name_history.push(search.name_text.clone());
        }
        if !search.contents_text.is_empty() && !ops.content_history.contains(&search.contents_text) {
            ops.content_history.push(search.contents_text.clone());
        }
        drop(ops);

        self.must_stop.store(false, Ordering::Relaxed);
        self.spawn_search(search);
    }

    pub fn save_and_quit(&self) {
        save_settings(&self.options.lock().unwrap());
        self.internal_sender.send(Message::Quit).expect("Quit");
    }

    pub fn save(&self) {
        save_settings(&self.options.lock().unwrap());
    }

    pub fn dir_is_valid(&self, dir: &str) -> bool {
        PathBuf::from(dir).exists()
    }

    pub fn set_options(&mut self, ops: Options) {
        *self.options.lock().unwrap() = ops;
    }

    pub fn get_options(&self) -> Options {
        self.options.lock().unwrap().clone()
    }

    pub fn set_sort(&mut self, sort: Sort) {
        self.options.lock().unwrap().sort = sort;
    }

    pub fn export(&self, paths: Vec<String>) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();

        let r = ctx.set_contents(paths.join("\n"));
        if let Err(err) = r {
            eprintln!("Clip error: {}", err);
        }
    }

    fn spawn_search(&self, search: Search) {
        let message_number = self.id;

        let file_sender = self.internal_sender.clone();
        //reset search, and send type
        let res = file_sender.send(Message::StartSearch(self.id));
        if let Result::Err(err) = res {
            eprintln!("Error sending {err}");
        }

        //change must stop to false, and reset counter
        self.counter.store(0, Ordering::Relaxed);
        self.must_stop.store(false, Ordering::Relaxed);

        //do name search
        let must_stop1 = self.must_stop.clone();
        let counter1 = self.counter.clone();
        let search1 = search.clone();
        let file_sender1 = file_sender.clone();
        let options1 = self.options.lock().unwrap().clone();

        if !search.name_text.is_empty() {
            thread::spawn(move || {
                let start = Instant::now();
                Manager::find_names(&search1, options1, message_number, file_sender1.clone(), must_stop1, counter1);
                if let Err(err) = file_sender1.send(Message::Done(message_number, start.elapsed())) {
                    eprintln!("Manager: Could not send result {message_number} {err:?}");
                }
            });
        }

        //do content search (only if name is empty, otherwise it will be spawned after)
        let must_stop2 = self.must_stop.clone();
        let counter2 = self.counter.clone();
        let options2 = self.options.lock().unwrap().clone();
        if !search.contents_text.is_empty() && search.name_text.is_empty() {
            thread::spawn(move || {
                let start = Instant::now();
                let files = Manager::find_contents(&search.contents_text, &search.dir, None, options2, must_stop2, counter2);
                file_sender
                    .send(Message::ContentFiles(files.results, message_number, start.elapsed()))
                    .unwrap();
                file_sender.send(Message::FileErrors(files.errors)).unwrap();
                file_sender.send(Message::Done(message_number, start.elapsed())).unwrap();
            });
        }
    }

    fn find_names(search: &Search, options: Options, id: usize, file_sender: Sender<Message>, must_stop: Arc<AtomicBool>, counter: Arc<AtomicUsize>) {
        let text = &search.name_text;
        let dir = &search.dir;
        let ftype = options.name.file_types;
        let sens = options.name.case_sensitive;
        let re = regex::RegexBuilder::new(text).case_insensitive(!sens).build();
        if re.is_err() {
            return;
        }
        let re = re.unwrap();
        let re = Arc::new(re);

        let walker = WalkBuilder::new(dir)
            .follow_links(options.name.follow_links)
            .same_file_system(options.name.same_filesystem)
            .threads(num_cpus::get())
            .hidden(options.name.ignore_dot)
            .git_ignore(options.name.use_gitignore)
            .build_parallel();

        //walk dir
        walker.run(|| {
            let file_sender = file_sender.clone();
            let re = re.clone();

            let options = options.clone();
            let must_stop = must_stop.clone();
            let counter = counter.clone();
            Box::new(move |result| {
                if must_stop.load(Ordering::Relaxed) {
                    return ignore::WalkState::Quit;
                }
                let dent = match result {
                    Ok(dent) => dent,
                    Err(err) => {
                        let _ = file_sender.send(Message::FileErrors(vec![err.to_string()]));
                        return ignore::WalkState::Continue;
                    }
                };

                let fs_type = dent.file_type();
                if fs_type.is_none() {
                    return ignore::WalkState::Continue;
                }
                let fs_type = fs_type.unwrap();

                //skip files if we dont want them
                match ftype {
                    FTypes::Files => {
                        if !fs_type.is_file() {
                            return ignore::WalkState::Continue;
                        }
                    }
                    FTypes::Directories => {
                        if !fs_type.is_dir() {
                            return ignore::WalkState::Continue;
                        }
                    }
                    _ => (),
                }

                let is_match = re.clone().is_match(dent.file_name().to_str().unwrap_or_default());

                if is_match {
                    let mut must_add = true;
                    let mut matches = vec![];
                    if !search.contents_text.is_empty() {
                        if fs_type.is_dir() {
                            must_add = false;
                        } else {
                            //check if contents match
                            let cont = Manager::find_contents(
                                &search.contents_text,
                                dir,
                                Some(HashSet::from_iter([dent.path().to_string_lossy().to_string()])),
                                options.clone(),
                                must_stop.clone(),
                                counter.clone(),
                            );
                            if cont.results.is_empty() {
                                must_add = false;
                            } else {
                                matches = cont.results[0].matches.clone();
                            }

                            if !cont.errors.is_empty() {
                                file_sender.send(Message::FileErrors(cont.errors)).unwrap();
                            }
                        }
                    }

                    if must_add {
                        let res = file_sender.send(Message::File(
                            FileInfo {
                                path: dent.path().to_string_lossy().to_string(),
                                name: dent.file_name().to_string_lossy().to_string(),
                                ext: PathBuf::from(dent.path())
                                    .extension()
                                    .unwrap_or(&OsString::from(""))
                                    .to_str()
                                    .unwrap_or_default()
                                    .into(),
                                matches,
                                is_folder: dent.file_type().unwrap().is_dir(),
                                plugin: None,
                            },
                            id,
                        ));
                        if let Result::Err(err) = res {
                            eprintln!("Error sending {err}");
                        }
                    }
                }

                ignore::WalkState::Continue
            })
        });
    }

    fn find_contents(
        text: &str,
        dir: &str,
        allowed_files: Option<HashSet<String>>,
        options: Options,
        must_stop: Arc<AtomicBool>,
        counter: Arc<AtomicUsize>,
    ) -> ContentFileInfoResults {
        let content_results = rgtools::search_contents(
            text,
            &[OsString::from_str(dir).unwrap()],
            allowed_files,
            options.content,
            must_stop,
            counter,
        );
        let strings = content_results.results;
        let errors = content_results.errors;

        let file_line_content: Vec<Vec<&str>> = strings
            .iter()
            .map(|x| x.split(&SEPARATOR).collect::<Vec<&str>>())
            .filter(|x| x.len() == 3)
            .collect();
        let mut hm: HashMap<String, FileInfo> = HashMap::new();
        for f in file_line_content.iter() {
            let (path, extended): (String, Option<ExtendedType>) = match f[0].split_once(EXTENSION_SEPARATOR) {
                Some((a, b)) => (a.to_string(), Some(b.into())),
                None => (f[0].to_string(), None),
            };
            let pb = PathBuf::from(&path);

            let entry = hm.entry(path.clone()).or_insert(FileInfo {
                path: path.clone(),
                matches: vec![],
                ext: pb.extension().unwrap_or(&OsString::from("")).to_str().unwrap_or_default().into(),
                name: PathBuf::from(f[0]).file_name().unwrap_or_default().to_str().unwrap_or_default().into(),
                is_folder: pb.is_dir(),
                plugin: extended,
            });
            entry.matches.push(Match {
                line: f[1].parse().unwrap_or(0),
                content: f[2].to_owned(),
            });
        }
        ContentFileInfoResults {
            results: hm.into_values().collect(),
            errors,
        }
    }

    pub fn do_sort(vec: &mut [FileInfo], sort: Sort) {
        match sort {
            Sort::None => (),
            Sort::Path => vec.sort_by(|a, b| a.path.cmp(&b.path)),
            Sort::Name => vec.sort_by(|a, b| a.name.cmp(&b.name)),
            Sort::Extension => vec.sort_by(|a, b| a.ext.cmp(&b.ext)),
        };
    }
}

#[derive(Default)]
pub struct ContentFileInfoResults {
    pub results: Vec<FileInfo>,
    pub errors: Vec<String>,
}

fn message_receiver(internal_receiver: Receiver<Message>, external_sender: Sender<SearchResult>, ops: Arc<Mutex<Options>>) {
    let mut final_names = vec![];
    let mut latest_number = 0;
    let mut tot_elapsed = Duration::from_secs(0);
    loop {
        let message = internal_receiver.recv();
        if message.is_err() {
            continue;
        }
        let message = message.unwrap();
        match message {
            Message::StartSearch(id) => {
                latest_number = id;
                tot_elapsed = Duration::from_secs(0);
                final_names.clear();
            }
            Message::ContentFiles(files, number, elapsed) => {
                if number != latest_number {
                    return;
                }
                //only update if new update (old updates are discarded)
                for f in files {
                    final_names.push(f);
                }
                tot_elapsed += elapsed;
            }
            Message::File(file, number) => {
                //only update if new update (old updates are discarded)
                if number != latest_number {
                    return;
                }
                //send to output
                final_names.push(file.clone());
                //quietly ignore if no receiver because probably closed
                let _ = external_sender.send(SearchResult::InterimResult(file));
            }
            Message::Done(number, elapsed) => {
                if number != latest_number {
                    return;
                }
                tot_elapsed += elapsed.to_owned();

                let sort_type = ops.lock().unwrap().sort;
                Manager::do_sort(&mut final_names, sort_type);
                let results = SearchResult::FinalResults(FinalResults {
                    id: latest_number,
                    data: final_names.to_vec(),
                    duration: tot_elapsed,
                });

                //send out to whoever is listening
                external_sender.send(results).expect("Sent results");
            }

            Message::Quit => break,
            Message::FileErrors(err) => {
                // eprintln!("Err: {err:?}");
                external_sender.send(SearchResult::SearchErrors(err)).unwrap()
            }
        }
    }
}

fn get_or_create_settings_path() -> Option<String> {
    if let Some(mut dir) = dirs::config_dir() {
        dir.push("rusl");
        if dir.exists() {
            dir.push("config.toml");
            if dir.exists() {
                if let Some(file) = dir.to_str() {
                    return Some(file.to_string());
                }
            }
        }
    }
    //try create
    if let Some(mut dir) = dirs::config_dir() {
        dir.push("rusl");

        if std::fs::create_dir_all(&dir).is_ok() {
            dir.push("config.toml");
            if std::fs::File::create(&dir).is_ok() {
                if let Some(file) = dir.to_str() {
                    return Some(file.to_string());
                }
            }
        }
    }
    //else not able to
    None
}

fn load_options() -> Options {
    let mut ops = Options::default();

    if let Some(file) = get_or_create_settings_path() {
        if let Ok(data) = std::fs::read_to_string(file) {
            if let Ok(new_ops) = toml::from_str(&data) {
                ops = new_ops;
            } else {
                eprintln!("Could not load settings");
            }
        } else {
            eprintln!("Could read settings");
        }
    } else {
        eprintln!("Could not access settings file");
    }

    ops
}

fn save_settings(ops: &Options) {
    let mut save_ops = ops.clone();
    //only save last few history
    let count = 20;
    save_ops.name_history = save_ops.name_history.into_iter().rev().take(count).rev().collect();
    save_ops.content_history = save_ops.content_history.into_iter().rev().take(count).rev().collect();

    if let Some(file) = get_or_create_settings_path() {
        let result = toml::to_string_pretty(&save_ops);
        if let Ok(data) = result {
            let write_result = std::fs::write(file, data);
            if write_result.is_err() {
                eprintln!("Could not write settings {write_result:?}");
            }
        } else {
            eprintln!("Could not convert {result:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;

    #[test]
    fn find_names() {
        let mut dir = std::env::temp_dir();
        println!("Using Temporary directory: {}", dir.display());

        //create new directory in here, and create a file with the relevant text
        dir.push("rusltestdir");
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        if std::fs::create_dir_all(&dir).is_err() {
            panic!("could not create temp dir");
        }

        let mut file1 = dir.clone();
        file1.push("temp.csv");

        if std::fs::write(&file1, "hello\nthere 41 go").is_err() {
            panic!("could not create file");
        }

        let (s, r) = channel();
        let mut man = Manager::new(s);
        man.search(Search {
            dir: file1.to_string_lossy().to_string(),
            name_text: "temp.csv".to_string(),
            contents_text: "41".to_string(),
        });

        if let Ok(mess) = r.recv() {
            println!("{mess:?}");
            match mess {
                SearchResult::FinalResults(_) => panic!(),
                SearchResult::InterimResult(fi) => {
                    assert_eq!(fi.matches.len(), 1);
                }
                SearchResult::SearchErrors(_) => panic!(),
            }
        }
    }
}
