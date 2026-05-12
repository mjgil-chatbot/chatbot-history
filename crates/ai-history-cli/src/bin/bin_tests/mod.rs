use super::execute_command;
use ai_history_cli::Commands;
use ai_history_core::{BranchId, CheckpointId, SessionId, TurnId};
use ai_history_store::sqlite_store::{BranchInfo, CheckpointInfo, SqliteStore, TurnInfo};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
pub(super) struct TestContext {
    pub(super) store: SqliteStore,
    _temp_dir: TempDir,
    workspace: PathBuf,
}
impl TestContext {
    pub(super) fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        Self {
            store: SqliteStore::in_memory().unwrap(),
            _temp_dir: temp_dir,
            workspace,
        }
    }}
impl TestContext {
    pub(super) fn workspace(&self) -> &PathBuf {
        &self.workspace
    }}
impl TestContext {
    pub(super) fn workspace_str(&self) -> &str {
        self.workspace.to_str().unwrap()
    }}
impl TestContext {
    pub(super) fn run(&mut self, command: &Commands) -> Result<String, String> {
        execute_command(command, &self.workspace, &mut self.store)
    }}
impl TestContext {
    pub(super) fn create_session(&mut self) -> SessionId {
        let session_id = SessionId::new();
        self.store
            .create_session(session_id, self.workspace_str())
            .unwrap();
        session_id
    }}
impl TestContext {
    pub(super) fn create_main_branch(&mut self, session_id: SessionId) -> BranchId {
        let branch_id = BranchId::new();
        self.store
            .create_branch(&BranchInfo {
                id: branch_id,
                session_id,
                name: "main".to_string(),
                parent_branch_id: None,
                created_from_turn_id: None,
                head_turn_id: None,
                ancestry: Vec::new(),
                protected: false,
                archived: false,
                abandoned_future: false,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();
        branch_id
    }}
impl TestContext {
    pub(super) fn create_turn(
        &mut self,
        session_id: SessionId,
        branch_id: BranchId,
        parent_turn_id: Option<TurnId>,
        actor: &str,
        created_at: &str,
    ) -> TurnId {
        let turn_id = TurnId::new();
        self.store
            .create_turn(&TurnInfo {
                id: turn_id,
                session_id,
                branch_id,
                parent_turn_id,
                actor: actor.to_string(),
                created_at: created_at.to_string(),
                finalized_at: None,
            })
            .unwrap();
        turn_id
    }}
impl TestContext {
    pub(super) fn create_checkpoint(
        &mut self,
        session_id: SessionId,
        branch_id: BranchId,
        turn_id: Option<TurnId>,
        reason: &str,
    ) -> CheckpointId {
        let checkpoint_id = CheckpointId::new();
        let manifest_id = ai_history_core::ManifestId::new();
        self.store
            .create_checkpoint(&CheckpointInfo {
                id: checkpoint_id,
                session_id,
                branch_id,
                turn_id,
                manifest_id,
                reason: reason.to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            })
            .unwrap();
        checkpoint_id
    }}
impl TestContext {
    pub(super) fn write_file(&self, relative_path: &str, contents: &[u8]) {
        let path = self.workspace.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }}
impl TestContext {
    pub(super) fn output_path(&self, filename: &str) -> String {
        self.workspace.join(filename).to_string_lossy().into_owned()
    }}
