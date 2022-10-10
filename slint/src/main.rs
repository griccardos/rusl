//hide windows console
#![windows_subsystem = "windows"]

use librusl::manager::{Manager, SearchResult};
use librusl::options::{FTypes, Options, Sort};
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
    let foptions = Arc::new(Mutex::new(Options::default()));
    set_options(weak.clone(), foptions.clone());

    //spawn result receiver
    let weak_receiver = weak.clone();
    thread::spawn(move || {
        let weak = weak_receiver.clone();
        loop {
            if let SearchResult::FinalResults(res) = result_receiver.recv().unwrap() {
                set_data(weak.clone(), res.data.iter().map(|x| x.to_owned()).collect(), res.duration, false)
            }
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

        //options
        get_and_update_options(foptions.clone(), weak_edited.clone());

        let search = Search {
            name_text: name_text.to_string(),
            contents_text: content_text.to_string(),
            dir: dir.to_string(),
        };
        let mut manager = manager_search.lock().unwrap();
        manager.search(search);
    });

    //on change sort method, we resort
    let weak_sort = weak.clone();
    let _manager_sort = manager.clone();
    mw.on_sort_changed(move || {
        let weak = weak_sort.unwrap();
        let sort_new = weak.get_selected_sort().as_str().to_owned();
        let _sort_new = match sort_new.as_str() {
            "Path" => Sort::Path,
            "Extension" => Sort::Extension,
            "Name" => Sort::Name,
            _ => Sort::Path,
        };
        //TODO
        //let manager_sort = manager_sort.lock().unwrap();
        //manager_sort.set_sort(sort_new);
        //manager_sort.sort();
        //set_data(weak_sort.clone(), results.to_vec(), Duration::from_secs(0), false);
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
}

fn get_and_update_options(foptions: Arc<Mutex<Options>>, weak: Weak<MainWindow>) {
    let mut ops = foptions.lock().unwrap();
    let weak = weak.unwrap();
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
}

fn set_data(weak: Weak<MainWindow>, files: Vec<FileInfo>, elapsed: Duration, reset: bool) {
    let _ = weak.upgrade_in_event_loop(move |weak| {
        let count = files.len() as i32;
        let sfiles: Vec<SFileInfo> = files
            .iter()
            .map(|x| SFileInfo {
                name: x.path.clone().into(),
                data: x.content.clone().into(),
            })
            .collect();
        let model = VecModel::<SFileInfo>::from(sfiles);
        let modelrc = ModelRc::new(model);
        weak.set_files(modelrc);
        if reset {
            weak.set_message("...searching...".into());
        } else {
            weak.set_message(format!("Found {count} in {}s", elapsed.as_secs_f64()).into());
        }

        match count > 0 {
            true => weak.set_export_enabled(true),
            false => weak.set_export_enabled(false),
        }
    });
}

fn set_options(weak: Weak<MainWindow>, foptions: Arc<Mutex<Options>>) {
    let _ = weak.upgrade_in_event_loop(move |weak| {
        let ops = foptions.lock().unwrap();
        weak.set_case_sensitive(ops.name.case_sensitive);
        weak.set_content_case_sensitive(ops.content.case_sensitive);
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
