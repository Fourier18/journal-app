use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

use crate::{
    storage::{Entry, EntrySummary, SearchHit, SearchOptions, Vault},
    AppState,
};

fn journal_path(app: &AppHandle) -> PathBuf {
    let base = app
        .path()
        .document_dir()
        .unwrap_or_else(|_| dirs::document_dir().unwrap_or_default());
    base.join("Journal")
}

// ── Status ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultStatus {
    NoVault,   // first run — nothing on disk yet
    Locked,    // salt file exists but vault not unlocked
    Unlocked,  // vault is open and ready
}

#[tauri::command]
pub fn vault_status(app: AppHandle, state: State<'_, AppState>) -> VaultStatus {
    let guard = state.vault.lock().unwrap();
    if guard.is_some() {
        VaultStatus::Unlocked
    } else {
        let salt = journal_path(&app).join(".journal").join("salt");
        if salt.exists() {
            VaultStatus::Locked
        } else {
            VaultStatus::NoVault
        }
    }
}

// ── Lifecycle ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    password: String,
) -> Result<(), String> {
    let path = journal_path(&app);
    let vault = Vault::create(&path, &password).map_err(|e| e.to_string())?;
    *state.vault.lock().unwrap() = Some(vault);
    Ok(())
}

#[tauri::command]
pub fn unlock_vault(
    app: AppHandle,
    state: State<'_, AppState>,
    password: String,
) -> Result<(), String> {
    let path = journal_path(&app);
    let vault = Vault::unlock(&path, &password).map_err(|e| e.to_string())?;
    *state.vault.lock().unwrap() = Some(vault);
    Ok(())
}

#[tauri::command]
pub fn lock_vault(state: State<'_, AppState>) {
    *state.vault.lock().unwrap() = None; // drops Vault → key zeroized
}

// ── Entry operations ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct EntryWithBody {
    pub entry: Entry,
    pub body: String,
}

#[tauri::command]
pub fn list_entries(state: State<'_, AppState>) -> Result<Vec<EntrySummary>, String> {
    let guard = state.vault.lock().unwrap();
    let vault = guard.as_ref().ok_or("Vault is locked")?;
    vault.list_entries().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_entries(
    state: State<'_, AppState>,
    options: SearchOptions,
) -> Result<Vec<SearchHit>, String> {
    let guard = state.vault.lock().unwrap();
    let vault = guard.as_ref().ok_or("Vault is locked")?;
    vault.search(&options).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_entry(
    state: State<'_, AppState>,
    entry: Entry,
    body: String,
) -> Result<(), String> {
    let mut guard = state.vault.lock().unwrap();
    let vault = guard.as_mut().ok_or("Vault is locked")?;
    vault.create_entry(&entry, &body).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_entry(state: State<'_, AppState>, id: String) -> Result<EntryWithBody, String> {
    let guard = state.vault.lock().unwrap();
    let vault = guard.as_ref().ok_or("Vault is locked")?;
    let (entry, body) = vault.read_entry(&id).map_err(|e| e.to_string())?;
    Ok(EntryWithBody { entry, body })
}

#[tauri::command]
pub fn update_entry(
    state: State<'_, AppState>,
    id: String,
    entry: Entry,
    body: String,
) -> Result<(), String> {
    let mut guard = state.vault.lock().unwrap();
    let vault = guard.as_mut().ok_or("Vault is locked")?;
    vault.update_entry(&id, &entry, &body).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_entry(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut guard = state.vault.lock().unwrap();
    let vault = guard.as_mut().ok_or("Vault is locked")?;
    vault.delete_entry(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_backlinks(state: State<'_, AppState>, id: String) -> Result<Vec<EntrySummary>, String> {
    let guard = state.vault.lock().unwrap();
    let vault = guard.as_ref().ok_or("Vault is locked")?;
    vault.get_backlinks(&id).map_err(|e| e.to_string())
}
