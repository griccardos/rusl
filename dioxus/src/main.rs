//hide windows console
#![windows_subsystem = "windows"]

use std::{
    sync::{mpsc},
};

use dioxus::prelude:: *;

use librusl::{fileinfo::FileInfo, manager::Manager,  search::Search};


pub fn main() {
    // let props = Arc::new(Mutex::new(props));
    // let props2 = props.clone();
    // let props3 = props.clone();
    //
    // thread::spawn(move || loop {
    //     let mut p = props3.lock().unwrap();
    //     p.name.push_str(" and ");
    //     sender.send(DioxusMessage::Set(p.name));
    //     drop(p);
    //     thread::sleep(Duration::from_secs(1));
    // });
    // dioxus::desktop::launch_with_props(app, props2, |c| c);
    dioxus::desktop::launch(app);
}

fn app(cx: Scope<()>) -> Element {
    let text_name = use_state(&cx, || "".to_string());
    let text_dir = use_state(&cx, || ".".to_string());
    let message = use_state(&cx, || "Started".to_string());
    let data = use_state(&cx, || Vec::<FileInfo>::new());
    let (s, _r) = mpsc::channel();
    let man = use_ref(&cx, || Manager::new(s));
    //let ops = use_state(&cx, || BothOptions::default());

    // cx.spawn({
    //     async move {
    //         loop {
    //             let files = r.recv().unwrap();
    //             eprintln!("{:?}", files.data);
    //             sleep(Duration::from_millis(100)).await
    //         }
    //     }
    // });

    cx.render(rsx!(
        
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
                eprintln!("test");
                message.set(format!("found {}",data.len()));
            },"Find"}
            ul{
                data.iter().map(|x|rsx!{li{a{"{x.name}"}}})
            }
        }

    ))
}
