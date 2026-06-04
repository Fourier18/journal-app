import { useEffect, useMemo, useState } from "react";
import { format, parseISO } from "date-fns";
import { lockVault, searchEntries, type EntrySummary } from "../lib/commands";
import { useVaultStore, type Theme } from "../store/vault";
import "./Sidebar.css";

export default function Sidebar() {
  const { entries, selectedId, theme, setSelectedId, setStatus, setTheme, setShowNewEntryModal } =
    useVaultStore();

  const [query, setQuery] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [results, setResults] = useState<EntrySummary[] | null>(null);

  // The set of all tags across every entry, sorted, for the filter row.
  const tagUniverse = useMemo(() => {
    const set = new Set<string>();
    for (const e of entries) for (const t of e.tags ?? []) set.add(t);
    return Array.from(set).sort();
  }, [entries]);

  const filterActive = query.trim() !== "" || selectedTags.length > 0;

  // Run search (debounced) whenever the query, tags, or entry set changes.
  useEffect(() => {
    if (!filterActive) {
      setResults(null);
      return;
    }
    const handle = setTimeout(() => {
      searchEntries(query.trim(), selectedTags).then(setResults).catch(() => setResults([]));
    }, 200);
    return () => clearTimeout(handle);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, selectedTags, entries]);

  function toggleTag(tag: string) {
    setSelectedTags((prev) =>
      prev.includes(tag) ? prev.filter((t) => t !== tag) : [...prev, tag]
    );
  }

  function clearFilters() {
    setQuery("");
    setSelectedTags([]);
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

      {/* Search + tag filter */}
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
            <button className="search-clear" title="Clear" onClick={clearFilters}>
              ×
            </button>
          )}
        </div>
        {tagUniverse.length > 0 && (
          <div className="filter-tags">
            {tagUniverse.map((tag) => (
              <button
                key={tag}
                className={`filter-tag${selectedTags.includes(tag) ? " active" : ""}`}
                onClick={() => toggleTag(tag)}
              >
                #{tag}
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="entry-list">
        {entries.length === 0 && (
          <p className="empty-hint">No entries yet. Click ✏️ to create your first one.</p>
        )}
        {entries.length > 0 && displayed.length === 0 && (
          <p className="empty-hint">No entries match your search.</p>
        )}
        {displayed.map((e) => {
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
