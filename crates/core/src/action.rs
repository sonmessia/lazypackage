use crate::domain::{BackendKind, Package, PackageId};
use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum Action {
    KeyPressed(KeyEvent),
    SearchChanged(String),
    ToggleSearchScope,
    SetSearchScope(crate::domain::SearchScope),
    ExecuteSearch,
    BackendResult(BackendKind, Result<Vec<Package>, String>),
    SearchResult(BackendKind, Result<Vec<Package>, String>),
    InstallRequested(PackageId),
    RemoveRequested(PackageId),
    UpgradeRequested(PackageId),
    // App-level events
    Tick,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendOp {
    ListInstalled,
    Search(String),
    Install(PackageId),
    Remove(PackageId),
}

#[derive(Debug, Clone)]
pub enum Command {
    RunBackend { backend: BackendKind, op: BackendOp },
    Quit,
}
