mod capture;
mod cli;
mod clock;
mod config;
mod encryption;
mod input;
mod process;
mod recorder;
mod storage;

use cli::render_status;
use config::{AppConfig, Command};
use recorder::Recorder;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (command, config) = match AppConfig::from_args(&args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("configuration error: {:?}", error);
            std::process::exit(2);
        }
    };
    let mut recorder = Recorder::new(config);
    let session_root = Path::new("data").join("sessions");

    match command {
        Command::Start => {
            if let Err(error) = recorder.start() {
                eprintln!("recording start blocked: {:?}", error);
                std::process::exit(3);
            }

            if let Err(error) = recorder.persist_session(&session_root) {
                eprintln!("session persistence error: {:?}", error);
                std::process::exit(4);
            }
        }
        Command::Status => {}
        Command::Stop => {
            recorder.stop();
            if recorder.has_session_data() {
                if let Err(error) = recorder.persist_session(&session_root) {
                    eprintln!("session persistence error: {:?}", error);
                    std::process::exit(4);
                }
            }
        }
    }

    println!("{}", render_status(&recorder.status_snapshot()));
}
