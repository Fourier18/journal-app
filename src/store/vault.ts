import { create } from "zustand";
import type { EntrySummary, VaultStatus } from "../lib/commands";

export type Theme = "light" | "dark" | "sepia";

interface VaultStore {
  status: VaultStatus;
  entries: EntrySummary[];
  selectedId: string | null;
  theme: Theme;

  setStatus: (s: VaultStatus) => void;
  setEntries: (e: EntrySummary[]) => void;
  setSelectedId: (id: string | null) => void;
  setTheme: (t: Theme) => void;
}

export const useVaultStore = create<VaultStore>((set) => ({
  status: "locked",
  entries: [],
  selectedId: null,
  theme: "light",

  setStatus: (status) => set({ status }),
  setEntries: (entries) => set({ entries }),
  setSelectedId: (selectedId) => set({ selectedId }),
  setTheme: (theme) => {
    document.documentElement.setAttribute("data-theme", theme === "light" ? "" : theme);
    set({ theme });
  },
}));
