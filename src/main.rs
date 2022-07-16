//hide windows console
//#![windows_subsystem = "windows"]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fileinfo;
mod mainegui;
mod manager;
mod options;
mod rgtools;
mod search;

fn main() {
    mainegui::run_egui();
}
