use std::process::exit;

mod input;
mod pin;
mod render;
mod cli;

use clap::Parser;
use input::Input;
use crossterm::terminal;
use pin::Document;
use input::KeyMap;
use cli::Args;
fn main() {
    let args = Args::parse();
    let mut doc = Document::open(&args.text).unwrap();
    let key_map = KeyMap::open(&args.keymap).unwrap();
    loop {
        let restart = Input::new(&mut doc, &key_map, args.pinyin).run();
        if !restart {
            break;
        }
    }
}

fn die() {
    if terminal::is_raw_mode_enabled().expect("Can not read if raw mode is enabled") {
        terminal::disable_raw_mode().expect("Failed to disable raw mode");
    }
    exit(0);
}
