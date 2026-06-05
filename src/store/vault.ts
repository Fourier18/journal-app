import { create } from "zustand";
import type { EntrySummary, VaultStatus } from "../lib/commands";

export type Theme = "light" | "dark" | "sepia";

interface VaultStore {
  status: VaultStatus;
  entries: EntrySummary[];
  selectedId: string | null;
  theme: Theme;
  showNewEntryModal: boolean;

  setStatus: (s: VaultStatus) => void;
  setEntries: (e: EntrySummary[]) => void;
  patchEntry: (id: string, patch: Partial<EntrySummary>) => void;
  setSelectedId: (id: string | null) => void;
  setTheme: (t: Theme) => void;
  setShowNewEntryModal: (show: boolean) => void;
}

export const useVaultStore = create<VaultStore>((set) => ({
  status: "locked",
  entries: [],
  selectedId: null,
  theme: "light",
  showNewEntryModal: false,

  setStatus: (status) => set({ status }),
  setEntries: (entries) => set({ entries }),
  patchEntry: (id, patch) =>
    set((s) => ({
      entries: s.entries.map((e) => (e.id === id ? { ...e, ...patch } : e)),
    })),
  setSelectedId: (selectedId) => set({ selectedId }),
  setShowNewEntryModal: (showNewEntryModal) => set({ showNewEntryModal }),
  setTheme: (theme) => {
    document.documentElement.setAttribute(
      "data-theme",
      theme === "light" ? "" : theme
    );
    set({ theme });
  },
}));
