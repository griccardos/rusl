//hide windows console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fileinfo;
mod maindruid;
mod manager;
mod options;
mod rgtools;
mod search;

fn main() {
    maindruid::run_druid();
}
