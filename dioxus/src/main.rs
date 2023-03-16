//hide windows console
#![windows_subsystem = "windows"]

use std::{cell::RefCell, sync::mpsc, time::Duration};

use dioxus::prelude::*;

use dioxus_desktop::{tao::window::Icon, Config, WindowBuilder};
use librusl::{fileinfo::FileInfo, manager::Manager, search::Search};

pub fn main() {
    let mut html = include_str!("../index.html").to_string();
    html = html.replace("CUSTOM_CSS", include_str!("../mui.min.css"));
    html = html.replace("CUSTOM_JS", include_str!("../mui.min.js"));

    dioxus_desktop::launch_cfg(
        app,
        Config::new()
            .with_window(WindowBuilder::new().with_title("rusl").with_window_icon(Some(load_icon())))
            .with_custom_index(html),
    );
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

fn app(cx: Scope<()>) -> Element {
    let text_name = use_state(&cx, || "".to_string());
    let text_contents = use_state(&cx, || "".to_string());
    let text_dir = use_state(&cx, || ".".to_string());
    let message = use_state(&cx, || "".to_string());
    let data = use_state(&cx, || RefCell::new(Vec::<FileInfo>::new()));
    let (s, r) = mpsc::channel();
    let man = use_ref(&cx, || Manager::new(s));
    let count = use_state(cx, || 0);

    //run in background
    // loop, if find then output, else sleep for a while
    use_coroutine(&cx, |_: UnboundedReceiver<()>| {
        let data = data.clone();
        let message = message.clone();
        let count = count.clone();
        async move {
            loop {
                match r.try_recv() {
                    Ok(files) => {
                        match files {
                            librusl::manager::SearchResult::FinalResults(fe) => {
                                eprintln!("Found {}", fe.data.len());
                                let current = data.current();
                                let mut mutable = current.borrow_mut();
                                mutable.clear();
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
                                    });
                                };
                                message.set(format!("Found {} in {:.2}s", found_count, fe.duration.as_secs_f32()));
                            }
                            librusl::manager::SearchResult::InterimResult(ir) => {
                                count.set(*count.current() + 1);

                                let current = data.current();
                                let mut mutable = current.borrow_mut();
                                let mes = format!("Found {}, searching...", count.current());
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
                                    });
                                }
                            }
                            librusl::manager::SearchResult::SearchErrors(_) => { /*todo show errors*/ }
                        }
                    }

                    Err(_) => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        }
    });

    cx.render(rsx!(

            div{


                //toolbar
                div{
                    style:"background-color:#404040;padding:10px;",
                div{
                    class:"mui-textfield",

                    input{
                        style:"color:lightgray;",
                    value:"{text_name}",
                    placeholder:"Regex file name search e.g. ^mai.*rs$ or r.st or ^best",
                    oninput: move|evt|{ let newval=evt.value.clone(); text_name.set(newval);},
                }
                label{
                    style:"color:white",
                    "File name"
                }
            }
            div{
                class:"mui-textfield",
                input{
                    style:"color:lightgray;",
                value:"{text_contents}",
                placeholder:"Regex content search e.g. str.{2}g",
                oninput: move|evt|{ let newval=evt.value.clone(); text_contents.set(newval);},
            }
            label{
                style:"color:white",
                "Contents"
            }
        }
            div{
                class:"mui-textfield",
                input{
                    style:"color:lightgray;",
                    value:"{text_dir}",
                    oninput: move|evt|{ let newval=evt.value.clone(); text_dir.set(newval);},
                },
                label{
                    style:"color:white",
                    "Directory"
                }
            }
            div{
                "{message}",
            }
            div{
            button{
                class:"mui-btn mui-btn--primary mui-btn--raised",
                onclick:move|_|{
                    if text_name.is_empty(){
                        message.set("Nothing to search for".to_string());
                    }else{
                message.set("Searching".to_string());
                man.with_mut(|x|x.search(Search {
                    name_text: text_name.to_string(),
                    contents_text:text_contents.to_string(),
                    dir: text_dir.to_string(),
                    ..Default::default()
                }));
            }


                },
                "Find"
            }
            }
        }

        //results
            div{
                style:"padding:10px",
                data.current().borrow().iter().map(|x|
                    {
                        rsx!(
                    div{
                        label{
                            style:if text_contents.is_empty(){""}else{ "color:blue;font-weight:bold"},
                            "{x.path}"
                        },
                    }
                    for mat in &x.matches{
                        rsx!(
                            div{
                            label{
                                style:"color:darkgreen;font-weight:bold",
                                "{mat.line}: "
                            }

                            label{"{limit_len(&mat.content, 100)}"},
                        }
                        )
                    }

                        )

            }
            )


            }

    }
    ))
}

fn limit_len(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        None => s,
        Some((i, _)) => &s[..i],
    }
}
