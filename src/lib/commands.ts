import { invoke } from "@tauri-apps/api/core";

export type VaultStatus = "no_vault" | "locked" | "unlocked";

export type EntryType = "daily" | "free_form";

// serde #[serde(untagged)] serialises Number(f64) → number, Text(String) → string
export type MetadataValue = number | string;

export interface Entry {
  id: string;
  created_at: string;
  updated_at: string;
  entry_type: EntryType;
  template: string;
  tags: string[];
  metadata: Record<string, MetadataValue>;
  title?: string;
}

export interface EntrySummary {
  id: string;
  file_path: string;
  entry_type: string;
  template: string;
  created_at: string;
  updated_at: string;
  title?: string;
}

export interface EntryWithBody {
  entry: Entry;
  body: string;
}

export const vaultStatus  = ()                                   => invoke<VaultStatus>("vault_status");
export const createVault  = (password: string)                   => invoke<void>("create_vault",  { password });
export const unlockVault  = (password: string)                   => invoke<void>("unlock_vault",  { password });
export const lockVault    = ()                                   => invoke<void>("lock_vault");
export const listEntries  = ()                                   => invoke<EntrySummary[]>("list_entries");
export const createEntry  = (entry: Entry, body: string)         => invoke<void>("create_entry",  { entry, body });
export const readEntry    = (id: string)                         => invoke<EntryWithBody>("read_entry", { id });
export const updateEntry  = (id: string, entry: Entry, body: string) => invoke<void>("update_entry", { id, entry, body });
export const deleteEntry  = (id: string)                         => invoke<void>("delete_entry",  { id });
