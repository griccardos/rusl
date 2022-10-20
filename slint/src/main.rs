//hide windows console
#![windows_subsystem = "windows"]

use librusl::manager::{Manager, SearchResult};
use librusl::options::{FTypes, Sort};
use librusl::search::Search;
use std::sync::mpsc;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use slint::{ModelRc, VecModel, Weak};

use librusl::fileinfo::FileInfo;
slint::include_modules!();

pub fn main() {
    // manager
    let (result_sender, result_receiver) = mpsc::channel();
    let manager = Arc::new(Mutex::new(Manager::new(result_sender.clone())));

    //results
    let results: Arc<Mutex<Vec<FileInfo>>> = Arc::new(Mutex::new(vec![]));

    //gui window
    let mw = MainWindow::new();
    let weak = mw.as_weak();

    set_options(weak.clone(), manager.clone());

    //spawn result receiver
    let weak_receiver = weak.clone();
    let results_receiver = results.clone();
    thread::spawn(move || {
        let weak = weak_receiver.clone();
        let mut current: Vec<FileInfo> = vec![];
        let mut counter = 0;
        loop {
            match result_receiver.recv().unwrap() {
                SearchResult::FinalResults(res) => {
                    set_data(weak.clone(), res.data.iter().map(|x| x.to_owned()).collect(), res.duration, true);
                    current.clear();
                    counter = 0;

                    *results_receiver.lock().unwrap() = res.data;
                }
                SearchResult::InterimResult(res) => {
                    counter += 1;
                    if current.len() < 1000 {
                        current.push(res);
                        set_data(
                            weak.clone(),
                            current.iter().map(|x| x.to_owned()).collect(),
                            Duration::from_secs(0),
                            false,
                        );
                    }
                    let _ = weak.upgrade_in_event_loop(move |weak| weak.set_message(format!("Found {counter} ...").into()));
                }
            };
        }
    });

    //on file search edited
    let weak_edited = weak.clone();
    let manager_search = manager.clone();
    mw.on_search(move || {
        let weak = weak_edited.clone().unwrap();
        let name_text = weak.get_find_text().as_str().to_string();
        let content_text = weak.get_content_find_text().as_str().to_string();
        let dir = weak.get_directory().as_str().to_owned();
        let search = Search {
            name_text: name_text.to_string(),
            contents_text: content_text.to_string(),
            dir: dir.to_string(),
        };

        get_and_update_options(manager_search.clone(), weak_edited.clone());

        let mut manager = manager_search.lock().unwrap();

        if name_text.is_empty() && content_text.is_empty() {
            weak.set_message("Nothing to search for".into());
            return;
        }

        if !manager.dir_is_valid(&search.dir) {
            weak.set_message("Invalid directory".into());
            return;
        }

        weak.set_message("Searching...".into());

        manager.search(search);
    });

    //on change sort method, we resort
    let weak_sort = weak.clone();
    let manager_sort = manager.clone();
    let results_sort = results.clone();
    mw.on_sort_changed(move || {
        println!("sort changed");
        let weak = weak_sort.unwrap();
        let sort_new = weak.get_selected_sort().as_str().to_owned();
        let sort_new = match sort_new.as_str() {
            "Path" => Sort::Path,
            "Extension" => Sort::Extension,
            "Name" => Sort::Name,
            "None" => Sort::None,
            _ => Sort::None,
        };
        //TODO
        let mut manager_sort = manager_sort.lock().unwrap();
        manager_sort.set_sort(sort_new);
        let mut results_vec = results_sort.lock().unwrap().to_vec();
        println!("about to sortt {}", results_vec.len());
        Manager::do_sort(&mut results_vec, sort_new);
        println!("sorted {}", results_vec.len());
        *results_sort.lock().unwrap() = results_vec.clone();

        set_data(weak_sort.clone(), results_vec, Duration::from_secs(0), false);
    });

    //exports
    let results_export_paths = results.clone();
    let manager_export = manager.clone();
    mw.on_export(move |typ: i32| {
        let results = results_export_paths.lock().unwrap();
        let results: Vec<String> = match typ.into() {
            ExportType::FullPath => results.iter().map(|x| x.path.clone()).collect(),
            ExportType::Name => results.iter().map(|x| x.name.clone()).collect(),
        };
        manager_export.lock().unwrap().export(results);
    });

    //dirchange
    let weak_dir_changed = weak.clone();
    mw.on_dir_changed(move || {
        let weak = weak_dir_changed.clone().unwrap();
        let dir = weak.get_directory().as_str().to_owned();
        if !PathBuf::from(dir).exists() {
            weak.set_error_dir(true);
        } else {
            weak.set_error_dir(false);
        }
    });

    //run window until quit
    mw.run();

    //save options
    manager.lock().unwrap().save_and_quit();
}

fn get_and_update_options(manager: Arc<Mutex<Manager>>, weak: Weak<MainWindow>) {
    let mut man = manager.lock().unwrap();
    let weak = weak.unwrap();
    let mut ops = man.get_options();
    //get name options
    ops.name.case_sensitive = weak.get_case_sensitive();
    let ftypes: &str = &weak.get_selected_ftypes().to_string();
    ops.name.file_types = match ftypes {
        "All" => FTypes::All,
        "Files" => FTypes::Files,
        "Directories" => FTypes::Directories,
        _ => FTypes::All,
    };

    //get content options
    ops.content.case_sensitive = weak.get_content_case_sensitive();
    man.set_options(ops);
}

fn set_data(weak: Weak<MainWindow>, files: Vec<FileInfo>, elapsed: Duration, finished: bool) {
    let _ = weak.upgrade_in_event_loop(move |weak| {
        let count = files.len() as i32;
        let mut sfiles: Vec<SFileInfo> = files
            .iter()
            .take(1000)
            .map(|x| SFileInfo {
                name: x.path.clone().into(),
                data: x.content().clone().into(),
            })
            .collect();
        if count > 1000 {
            sfiles.push(SFileInfo {
                data: "".into(),
                name: format!("...and {} others", count - 1000).into(),
            });
        }
        let model = VecModel::<SFileInfo>::from(sfiles);
        let modelrc = ModelRc::new(model);
        weak.set_files(modelrc);

        if finished {
            weak.set_message(format!("Found {count} in {:.3}s", elapsed.as_secs_f64()).into());
        }

        match count > 0 {
            true => weak.set_export_enabled(true),
            false => weak.set_export_enabled(false),
        }
    });
}

fn set_options(weak: Weak<MainWindow>, manager: Arc<Mutex<Manager>>) {
    let _ = weak.upgrade_in_event_loop(move |weak| {
        let man = manager.lock().unwrap();
        let ops = man.get_options();
        weak.set_case_sensitive(ops.name.case_sensitive);
        weak.set_content_case_sensitive(ops.content.case_sensitive);
        weak.set_directory(ops.last_dir.clone().into());
    });
}

enum ExportType {
    FullPath = 1,
    Name,
}

impl Into<ExportType> for i32 {
    fn into(self) -> ExportType {
        match self {
            x if x == ExportType::FullPath as i32 => ExportType::FullPath,
            x if x == ExportType::Name as i32 => ExportType::Name,
            _ => ExportType::FullPath,
        }
    }
}
