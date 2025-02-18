//hide windows console
#![windows_subsystem = "windows"]

use std::{sync::mpsc, time::Duration};

//use dioxus_desktop::{tao::window::Icon, Config, WindowBuilder};
use dioxus::{
    desktop::{tao::window::Icon, WindowBuilder},
    prelude::*,
};
use librusl::{fileinfo::FileInfo, manager::Manager, search::Search};
pub fn main() {
    let mut html = include_str!("../index.html").to_string();
    html = html.replace("CUSTOM_CSS", include_str!("../mui.min.css"));
    html = html.replace("CUSTOM_JS", include_str!("../mui.min.js"));

    LaunchBuilder::new()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_menu(None)
                .with_window(WindowBuilder::new().with_title("rusl").with_window_icon(Some(load_icon())))
                .with_custom_index(html),
        )
        .launch(app);
}

fn load_icon() -> Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(include_bytes!("icons/icon.png")).unwrap().into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap()
}
fn app() -> Element {
    let mut text_name = use_signal(|| "".to_string());
    let mut text_contents = use_signal(|| "".to_string());
    let mut text_dir = use_signal(|| ".".to_string());
    let mut message = use_signal(|| "".to_string());
    let data = use_signal(|| Vec::<FileInfo>::new());
    let (s, r) = mpsc::channel();
    let mut man = use_signal(|| Manager::new(s));
    let count = use_signal(|| 0);
    let r = use_signal(|| r);

    //run in background
    // loop, if find then output, else sleep for a while
    use_coroutine(move |_: UnboundedReceiver<()>| {
        let mut data = data.clone();
        let mut message = message.clone();
        let mut count = count.clone();
        async move {
            loop {
                match r.read().try_recv() {
                    Ok(files) => {
                        match files {
                            librusl::manager::SearchResult::FinalResults(fe) => {
                                eprintln!("Found {}", fe.data.len());
                                let current = data.clone();
                                let mut mutable = current.clone();
                                mutable.write().clear();
                                let found_count = fe.data.len();
                                if found_count < 1000 {
                                    mutable.extend(fe.data);
                                } else {
                                    mutable.extend(fe.data.into_iter().take(1000));
                                    mutable.push(FileInfo {
                                        path: format!("...and {} more", found_count - 1000),
                                        matches: vec![],
                                        ext: "".to_string(),
                                        name: "".to_string(),
                                        is_folder: false,
                                        plugin: None,
                                        ranges: vec![],
                                    });
                                };
                                message.set(format!("Found {} in {:.2}s", found_count, fe.duration.as_secs_f32()));
                            }
                            librusl::manager::SearchResult::InterimResult(ir) => {
                                let c = *count.read();
                                count.set(c + 1);

                                let mut mutable = data.write();
                                let mes = format!("Found {}, searching...", count.read());
                                //eprintln!("{mes}");
                                message.set(mes);

                                if mutable.len() < 1000 {
                                    mutable.push(ir);
                                } else if mutable.len() == 1000 {
                                    mutable.push(FileInfo {
                                        path: format!("...and others"),
                                        matches: vec![],
                                        ext: "".to_string(),
                                        name: "".to_string(),
                                        is_folder: false,
                                        plugin: None,
                                        ranges: vec![],
                                    });
                                }
                            }
                            librusl::manager::SearchResult::SearchErrors(_) => { /*todo show errors*/ }
                            librusl::manager::SearchResult::SearchCount(_) => {}
                        }
                    }

                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        }
    });

    rsx!(



            //toolbar
            div { style: "background-color:#404040;padding:10px;",
                div { class: "mui-textfield",

                    input {
                        style: "color:lightgray;",
                        value: "{text_name}",
                        placeholder: "Regex file name search e.g. ^mai.*rs$ or r.st or ^best",
                        oninput: move |evt| {
                            let newval = evt.value().clone();
                            text_name.set(newval);
                        }
                    }
                    label { style: "color:white", "File name" }
                }
                div { class: "mui-textfield",
                    input {
                        style: "color:lightgray;",
                        value: "{text_contents}",
                        placeholder: "Regex content search e.g. str.{2}g",
                        oninput: move |evt| {
                            let newval = evt.value().clone();
                            text_contents.set(newval);
                        }
                    }
                    label { style: "color:white", "Contents" }
                }
                div { class: "mui-textfield",
                    input {
                        style: "color:lightgray;",
                        value: "{text_dir}",
                        oninput: move |evt| {
                            let newval = evt.value().clone();
                            text_dir.set(newval);
                        }
                    }
                    label { style: "color:white", "Directory" }
                }
                div { "{message}" }
                div {
                    button {
                        class: "mui-btn mui-btn--primary mui-btn--raised",
                        onclick: move |_| {
                            if text_name.read().is_empty() {
                                message.set("Nothing to search for".to_string());
                            } else {
                                message.set("Searching".to_string());
                                man.with_mut(|x| {
                                    x.search(
                                        &Search {
                                            name_text: text_name.to_string(),
                                            contents_text: text_contents.to_string(),
                                            dir: text_dir.to_string(),
                                            ..Default::default()
                                        },
                                    )
                                });
                            }
                        },
                        "Find"
                    }
                }
            }


            div {

            //results
            div { for x in  data.read().iter()      {

                    div{
                        label{
                            style:if text_contents.read().is_empty(){""}else{ "color:blue;font-weight:bold"},
                            "{x.path}"
                        },
                    }
                    for mat in &x.matches{

                            div{
                            label{
                                style:"color:darkgreen;font-weight:bold",
                                "{mat.line}: "
                            }

                            label{"{limit_len(&mat.content, 100)}"},
                        }

                    }



            }

            }
        }
    )
}

fn limit_len(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        None => s,
        Some((i, _)) => &s[..i],
    }
}
