use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use clipboard::{ClipboardContext, ClipboardProvider};
use ignore::WalkBuilder;

use crate::fileinfo::FileInfo;
use crate::options::{FTypes, Options, Sort};
use crate::rgtools;
use crate::search::Search;

type Mid = usize;
pub enum Message {
    File(FileInfo, usize),
    Done(Mid, Duration),
    ContentFiles(Vec<FileInfo>, Mid, Duration),
    StartSearch,
    Quit,
}

pub enum SearchResult {
    FinalResults(FinalResults),
    InterimResult(FileInfo),
}
pub struct FinalResults {
    pub data: Vec<FileInfo>,
    pub duration: Duration,
    pub id: Mid,
}
pub struct Manager {
    internal_sender: Sender<Message>,
    id: Mid,
    pub options: Options,
    //results: Results,
}

impl Manager {
    pub fn new(external_sender: Sender<SearchResult>) -> Self {
        let ops = load_settings();

        //internal channel that sends results inside
        let (s, r) = std::sync::mpsc::channel();

        thread::spawn(move || {
            Manager::message_receiver(r, external_sender, Sort::Path);
        });
        Self {
            internal_sender: s,
            id: 0,
            options: ops,
            //  results: Results {
            //      data: vec![],
            //      duration: Duration::default(),
            //      id: 0,
            //  },
        }
    }

    pub fn search(&mut self, search: Search) {
        self.id += 1;
        self.options.last_dir = search.dir.clone();
        if !search.name_text.is_empty() && !self.options.name_history.contains(&search.name_text) {
            self.options.name_history.push(search.name_text.clone());
        }
        if !search.contents_text.is_empty() && !self.options.content_history.contains(&search.contents_text) {
            self.options.content_history.push(search.contents_text.clone());
        }
        self.spawn_search(search, self.internal_sender.clone(), self.options.clone(), self.id);
    }

    pub fn save_and_quit(&mut self) {
        save_settings(&mut self.options);
        self.internal_sender.send(Message::Quit).expect("Quit");
    }

    pub fn dir_is_valid(&self, dir: &str) -> bool {
        PathBuf::from(dir).exists()
    }

    pub fn set_options(&mut self, ops: Options) {
        self.options = ops;
    }

    pub fn export(&self, paths: Vec<String>) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();

        let r = ctx.set_contents(paths.join("\n"));
        if let Err(err) = r {
            eprintln!("Clip error: {}", err);
        }
    }

    fn spawn_search(&self, search: Search, file_sender: Sender<Message>, options: Options, message_number: usize) {
        //reset search, and send type
        file_sender.send(Message::StartSearch).unwrap();

        let file_sender_names = file_sender.clone();
        let file_sender_contents = file_sender.clone();
        //do name search
        if !search.name_text.is_empty() {
            Manager::spawn_find_names(search.clone(), file_sender_names, options.clone(), message_number);
        }

        //do content search (only if name is empty, otherwise it will be spawned after)
        if !search.contents_text.is_empty() && search.name_text.is_empty() {
            Manager::spawn_find_contents(search.clone(), file_sender_contents, options.clone(), None, message_number);
        }
    }

    fn find_names(search: &Search, options: Options, id: usize, file_sender: Sender<Message>) {
        let text = &search.name_text;
        let dir = &search.dir;
        let ftype = options.name.file_types.clone();
        let sens = options.name.case_sensitive.clone();
        let re = regex::RegexBuilder::new(text).case_insensitive(!sens).build();
        if re.is_err() {
            return;
        }
        let re = re.unwrap();
        let re = Arc::new(re);
        let re = re.clone();

        let walker = WalkBuilder::new(dir)
            .follow_links(options.name.follow_links)
            .same_file_system(options.name.same_filesystem)
            .threads(num_cpus::get())
            .build_parallel();

        //walk dir
        walker.run(|| {
            let file_sender = file_sender.clone();
            let re = re.clone();
            let ftype = ftype.clone();
            let options = options.clone();

            Box::new(move |result| {
                let dent = match result {
                    Ok(dent) => dent,
                    Err(err) => {
                        eprintln!("{}", err);
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
                    let mut content = "".to_string();
                    if !search.contents_text.is_empty() {
                        let cont = Manager::find_contents(
                            &search.contents_text,
                            dir,
                            Some(HashSet::from_iter([dent.path().to_string_lossy().to_string()])),
                            options.clone(),
                        );
                        if cont.is_empty() {
                            must_add = false;
                        } else {
                            content = cont.iter().next().unwrap().content.clone();
                        }
                    }

                    if must_add {
                        file_sender
                            .send(Message::File(
                                FileInfo {
                                    path: dent.path().to_string_lossy().to_string(),
                                    name: dent.file_name().to_string_lossy().to_string(),
                                    ext: PathBuf::from(dent.path())
                                        .extension()
                                        .unwrap_or(&OsString::from(""))
                                        .to_str()
                                        .unwrap_or_default()
                                        .into(),
                                    content: content,
                                },
                                id,
                            ))
                            .unwrap();
                    }
                }

                ignore::WalkState::Continue
            })
        });
    }

    fn find_contents(text: &str, dir: &str, allowed_files: Option<HashSet<String>>, options: Options) -> Vec<FileInfo> {
        let content_options = options.content.clone();
        let strings = rgtools::search(text, &[OsString::from_str(dir).unwrap()], allowed_files, content_options);

        const MAX: usize = 100;
        let result: Vec<FileInfo> = strings
            .iter()
            .filter_map(|x| x.split_once(&String::from_utf8(rgtools::SEPARATOR.to_vec()).unwrap()))
            .map(|x| {
                let mut data = x.1;
                //limit size
                if data.len() > MAX {
                    let mut count = MAX;
                    let mut datab = data.as_bytes();
                    loop {
                        datab = &datab[..count];
                        //in case in middle of grapheme
                        let temp = String::from_utf8(datab.to_vec());
                        if temp.is_ok() || count == 1 {
                            break;
                        }
                        count -= 1;
                    }
                    data = &data[..count]
                }
                (x.0, data)
            })
            .map(|x| FileInfo {
                path: x.0.into(),
                content: x.1.trim_start().into(),
                ext: PathBuf::from(x.0)
                    .extension()
                    .unwrap_or(&OsString::from(""))
                    .to_str()
                    .unwrap_or_default()
                    .into(),
                name: PathBuf::from(x.0).file_name().unwrap_or_default().to_str().unwrap_or_default().into(),
            })
            .collect();

        result
    }

    fn spawn_find_names(search: Search, file_sender: Sender<Message>, options: Options, message_number: usize) {
        thread::spawn(move || {
            let start = Instant::now();
            Manager::find_names(&search, options.clone(), message_number, file_sender.clone());
            file_sender.send(Message::Done(message_number, start.elapsed())).unwrap();
        });
    }

    fn spawn_find_contents(
        search: Search,
        file_sender: Sender<Message>,
        options: Options,
        allowed_files: Option<HashSet<String>>,
        message_number: usize,
    ) {
        thread::spawn(move || {
            let start = Instant::now();
            let files = Manager::find_contents(&search.contents_text, &search.dir, allowed_files, options);
            file_sender.send(Message::ContentFiles(files, message_number, start.elapsed())).unwrap();
            file_sender.send(Message::Done(message_number, start.elapsed())).unwrap();
        });
    }

    fn message_receiver(internal_receiver: Receiver<Message>, external_sender: Sender<SearchResult>, sort_type: Sort) {
        let mut final_names = HashMap::new();
        let mut latest_number = 0;
        let mut tot_elapsed = Duration::from_secs(0);
        loop {
            let message = internal_receiver.recv();
            if message.is_err() {
                eprintln!("unknown message");
                continue;
            }
            let message = message.unwrap();
            match message {
                Message::StartSearch => {
                    tot_elapsed = Duration::from_secs(0);
                    final_names.clear();
                }
                Message::ContentFiles(files, number, elapsed) => {
                    if number >= latest_number {
                        latest_number = number;
                        //only update if new update (old updates are discarded)
                        for f in files {
                            final_names.insert(f.path.to_owned(), f);
                        }
                        tot_elapsed += elapsed;
                    }
                }
                Message::File(file, number) => {
                    //only update if new update (old updates are discarded)
                    if number >= latest_number {
                        latest_number = number;

                        //send to output
                        final_names.insert(file.path.to_string(), file.clone());
                        external_sender.send(SearchResult::InterimResult(file)).unwrap();
                    }
                }
                Message::Done(_id, elapsed) => {
                    tot_elapsed += elapsed.to_owned();

                    let mut res = final_names.iter().map(|(_, v)| v.to_owned()).collect();
                    Manager::do_sort(&mut res, sort_type);
                    let results = SearchResult::FinalResults(FinalResults {
                        id: latest_number,
                        data: res.to_vec(),
                        duration: tot_elapsed,
                    });

                    //send out to whoever is listening
                    external_sender.send(results).expect("Sent results");
                }

                Message::Quit => break,
            }
        }
    }

    fn do_sort(vec: &mut Vec<FileInfo>, sort: Sort) {
        //always sort by path first...
        vec.sort_by(|a, b| a.path.cmp(&b.path));
        match sort {
            Sort::Path => (),
            Sort::Name => vec.sort_by(|a, b| a.name.cmp(&b.name)),
            Sort::Extension => vec.sort_by(|a, b| a.ext.cmp(&b.ext)),
        };
    }
}

fn get_or_create_settings_path() -> Option<String> {
    if let Some(dir) = dirs::config_dir() {
        let mut dir = dir.clone();
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
    if let Some(dir) = dirs::config_dir() {
        let mut dir = dir.clone();
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

fn load_settings() -> Options {
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

fn save_settings(ops: &mut Options) {
    if let Some(file) = get_or_create_settings_path() {
        let result = toml::to_string_pretty(ops);
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
