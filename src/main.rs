//hide windows console
#![windows_subsystem = "windows"]

mod fileinfo;
mod mainslint;
mod manager;
mod options;
mod rgtools;
mod search;

fn main() {
    mainslint::run_slint();
}
