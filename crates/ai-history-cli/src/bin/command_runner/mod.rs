use ai_history_cli::Commands;
use ai_history_store::sqlite_store::SqliteStore;
use std::path::PathBuf;

mod history;
mod maintenance;
mod session;

pub(crate) fn execute_command(
    command: &Commands,
    workspace: &PathBuf,
    store: &mut SqliteStore,
) -> Result<String, String> {
    match command {
        Commands::Init { title } => session::init(title, workspace, store),
        Commands::Checkpoint { name, reason } => {
            session::checkpoint(name, reason.as_str(), workspace, store)
        }
        Commands::ListCheckpoints { branch } => session::list_checkpoints(branch.as_str(), store),
        Commands::Restore {
            checkpoint_id,
            mode,
        } => session::restore(checkpoint_id.as_str(), mode, workspace, store),
        Commands::Fork {
            checkpoint_id,
            name,
        } => session::fork(checkpoint_id.as_str(), name.as_str(), store),
        Commands::Branches => session::branches(store),
        Commands::Diff {
            checkpoint_id,
            files_only,
        } => session::diff(checkpoint_id.as_str(), *files_only, workspace, store),
        Commands::Status => session::status(store),
        Commands::Gc { execute, protected } => {
            maintenance::gc(*execute, protected, workspace, store)
        }
        Commands::Doctor { fix } => maintenance::doctor(*fix, store),
        Commands::Recover { execute } => maintenance::recover(*execute, store),
        Commands::NavigateBack { count } => history::navigate_back(*count, store),
        Commands::NavigateForward { count } => history::navigate_forward(*count, store),
        Commands::ForkCurrent { name } => history::fork_current(name.as_str(), store),
        Commands::ForkFromCursor { turn_id, name } => {
            history::fork_from_cursor(turn_id.as_str(), name.as_str(), store)
        }
        Commands::ContinueFromCursor { turn_id, prompt } => {
            history::continue_from_cursor(turn_id.as_str(), prompt.as_str(), store)
        }
        Commands::Export {
            output,
            include_blobs,
        } => maintenance::export(output.as_str(), *include_blobs, store),
        Commands::Import { input, session_id } => {
            maintenance::import(input.as_str(), session_id.as_ref(), store)
        }
        Commands::Verify { checkpoint_id } => maintenance::verify(checkpoint_id.as_deref(), store),
    }
}
