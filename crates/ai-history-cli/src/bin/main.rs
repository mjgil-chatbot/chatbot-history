//! AI History CLI - Main entry point
//!
//! This binary wires CLI commands to real persistent storage APIs.

use ai_history_cli::{Cli, OutputFormatter};
use ai_history_store::sqlite_store::SqliteStore;
use clap::Parser;

#[cfg(test)]
mod bin_tests;
mod command_runner;

use command_runner::execute_command;

fn main() {
    let cli = Cli::parse();

    let store_path = cli.workspace.join(".ai-history").join("store.db");
    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let store = match SqliteStore::open(&store_path) {
        Ok(store) => store,
        Err(error) => {
            eprintln!(
                "{}",
                OutputFormatter::format_error(&format!("Failed to open store: {}", error), None)
            );
            std::process::exit(1);
        }
    };

    let mut store = store;
    let result = execute_command(&cli.command, &cli.workspace, &mut store);

    match result {
        Ok(message) => {
            if !message.is_empty() {
                println!("{}", message);
            }
        }
        Err(error) => {
            eprintln!("{}", OutputFormatter::format_error(&error, None));
            std::process::exit(1);
        }
    }
}
