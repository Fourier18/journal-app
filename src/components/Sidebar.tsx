import { format, parseISO } from "date-fns";
import { lockVault } from "../lib/commands";
import { useVaultStore, type Theme } from "../store/vault";
import "./Sidebar.css";

export default function Sidebar() {
  const { entries, selectedId, theme, setSelectedId, setStatus, setTheme, setShowNewEntryModal } =
    useVaultStore();

  function handleNewEntry() {
    setShowNewEntryModal(true);
  }

  async function handleLock() {
    await lockVault();
    setStatus("locked");
  }

  function cycleTheme() {
    const order: Theme[] = ["light", "dark", "sepia"];
    const next = order[(order.indexOf(theme) + 1) % order.length];
    setTheme(next);
  }

  const themeLabel: Record<Theme, string> = {
    light: "☀️",
    dark: "🌙",
    sepia: "📜",
  };

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <span className="sidebar-title">Journal</span>
        <div className="sidebar-actions">
          <button className="icon-btn" title="Cycle theme" onClick={cycleTheme}>
            {themeLabel[theme]}
          </button>
          <button className="icon-btn" title="New entry" onClick={handleNewEntry}>
            ✏️
          </button>
          <button className="icon-btn" title="Lock" onClick={handleLock}>
            🔒
          </button>
        </div>
      </div>

      <div className="entry-list">
        {entries.length === 0 && (
          <p className="empty-hint">No entries yet. Click ✏️ to create your first one.</p>
        )}
        {entries.map((e) => {
          const date = parseISO(e.created_at);
          const label = e.title || format(date, "MMMM d, yyyy");
          const sub = e.entry_type === "daily" ? "Daily entry" : "Free-form";
          return (
            <button
              key={e.id}
              className={`entry-item${e.id === selectedId ? " selected" : ""}`}
              onClick={() => setSelectedId(e.id)}
            >
              <span className="entry-label">{label}</span>
              <span className="entry-meta">{sub}</span>
            </button>
          );
        })}
      </div>
    </aside>
  );
}
