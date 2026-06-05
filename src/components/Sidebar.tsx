import { useEffect, useState } from "react";
import { format, parseISO } from "date-fns";
import { lockVault, searchEntries, type EntrySummary } from "../lib/commands";
import { useVaultStore, type Theme } from "../store/vault";
import "./Sidebar.css";

export default function Sidebar() {
  const { entries, selectedId, theme, setSelectedId, setStatus, setTheme, setShowNewEntryModal } =
    useVaultStore();

  const [query, setQuery] = useState("");
  const [results, setResults] = useState<EntrySummary[] | null>(null);

  const filterActive = query.trim() !== "";

  // Run text search (debounced) whenever the query or entry set changes.
  useEffect(() => {
    if (!filterActive) {
      setResults(null);
      return;
    }
    const handle = setTimeout(() => {
      searchEntries(query.trim(), []).then(setResults).catch(() => setResults([]));
    }, 200);
    return () => clearTimeout(handle);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, entries]);

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

  const displayed = results ?? entries;

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <span className="sidebar-title">Journal</span>
        <div className="sidebar-actions">
          <button className="icon-btn" title="Cycle theme" onClick={cycleTheme}>
            {themeLabel[theme]}
          </button>
          <button
            className="icon-btn"
            title="New entry"
            onClick={() => setShowNewEntryModal(true)}
          >
            ✏️
          </button>
          <button className="icon-btn" title="Lock" onClick={handleLock}>
            🔒
          </button>
        </div>
      </div>

      {/* Text search */}
      <div className="sidebar-search">
        <div className="search-input-wrap">
          <span className="search-icon">🔍</span>
          <input
            className="search-input"
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search entries…"
          />
          {filterActive && (
            <button className="search-clear" title="Clear" onClick={() => setQuery("")}>
              ×
            </button>
          )}
        </div>
      </div>

      <div className="entry-list">
        {entries.length === 0 && (
          <p className="empty-hint">No entries yet. Click ✏️ to create your first one.</p>
        )}
        {entries.length > 0 && displayed.length === 0 && (
          <p className="empty-hint">No entries match your search.</p>
        )}
        {displayed.map((e) => {
          const title = e.title?.trim() || "Untitled";
          const date = format(parseISO(e.created_at), "MMMM d, yyyy");
          return (
            <button
              key={e.id}
              className={`entry-item${e.id === selectedId ? " selected" : ""}`}
              onClick={() => setSelectedId(e.id)}
            >
              <span className="entry-label">{title}</span>
              <span className="entry-meta">{date}</span>
            </button>
          );
        })}
      </div>
    </aside>
  );
}
