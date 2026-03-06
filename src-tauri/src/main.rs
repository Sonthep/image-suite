// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // call example from the workspace renamer crate
    println!("renamer example: {}", renamer::example());

    // existing app entry
    image_renamer_lib::run();
}
