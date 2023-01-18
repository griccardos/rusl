//hide windows console
//#![windows_subsystem = "windows"]

use std::{cell::RefCell, sync::mpsc, time::Duration};

use dioxus::prelude::*;

use librusl::{fileinfo::FileInfo, manager::Manager, search::Search};

pub fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope<()>) -> Element {
    let text_name = use_state(&cx, || "".to_string());
    let text_dir = use_state(&cx, || ".".to_string());
    let message = use_state(&cx, || "Started".to_string());
    let data = use_state(&cx, || RefCell::new(Vec::<FileInfo>::new()));
    let (s, r) = mpsc::channel();
    let man = use_ref(&cx, || Manager::new(s));

    //run in background
    // loop, if find then output, else sleep for a while
    use_coroutine(&cx, |_: UnboundedReceiver<()>| {
        let data = data.clone();
        let message = message.clone();
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
                                 if found_count<1000{
                                mutable.extend(fe.data);
                                }else{
                                    
                                    mutable.extend(fe.data.into_iter().take(1000));
                                    mutable.push(FileInfo { path: format!("...and {} more",found_count-1000), matches: vec![], ext: "".to_string(), name: "".to_string(), is_folder: false });
                                };
                                message.set(format!("Found {} in {:.2}s", found_count, fe.duration.as_secs_f32()));
                            }
                            librusl::manager::SearchResult::InterimResult(ir) => {
                                let current = data.current();
                                let mut mutable = current.borrow_mut();
                                let mes = format!("Found {}, searching...", mutable.len());
                                //eprintln!("{mes}");
                                message.set(mes);
                               
                                if mutable.len()<1000{
                                    
                                mutable.push(ir);
                                }else if mutable.len()==1000{
                                    mutable.push(FileInfo { path: format!("...and others"), matches: vec![], ext: "".to_string(), name: "".to_string(), is_folder: false });
                                }
                            }
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
        head {
            link { rel: "stylesheet", href: "mui.min.css" }
        }
        body {
            div{
                class: "py-8 px-6",
                input{
                    placeholder:"File name",
                    value:"{text_name}",
                    oninput: move|evt|{ let newval=evt.value.clone(); text_name.set(newval);},
                }
                input{
                    placeholder:"Directory",
                    value:"{text_dir}",
                    oninput: move|evt|{ let newval=evt.value.clone(); text_dir.set(newval);},
                },
                "{message}",
            button{
                class:"inline-block w-full md:w-auto mx-3 my-3 px-2 py-1 font-medium text-white bg-indigo-500 hover:bg-indigo-600 rounded transition duration-200",
                onclick:move|_|{
                message.set("Searching".to_string());
                man.with_mut(|x|x.search(Search {
                    name_text: text_name.to_string(),
                    dir: text_dir.to_string(),
                    ..Default::default()
                }));
                
            },"Find"}
            ul{
                data.current().borrow().iter().map(|x|rsx!{li{a{"{x.path}"}}})
            }
        }
    }
    ))
}
