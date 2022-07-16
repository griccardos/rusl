//hide windows console
#![windows_subsystem = "windows"]

mod fileinfo;
mod maindioxus;
mod manager;
mod options;
mod rgtools;
mod search;

fn main() {
    maindioxus::run_dioxus();
}
