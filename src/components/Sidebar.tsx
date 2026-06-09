import { useEffect, useState } from "react";
import { format, parseISO } from "date-fns";
import { lockVault, searchEntries, type SearchHit, type SearchOptions } from "../lib/commands";
import { useVaultStore, type Theme } from "../store/vault";
import "./Sidebar.css";

const DEFAULT_OPTS: Omit<SearchOptions, "query"> = {
  in_body: true,
  in_title: true,
  in_tags: true,
  in_metadata: false,
  match_all_words: true,
  sort_by_relevance: true,
};

export default function Sidebar() {
  const {
    entries, selectedId, theme,
    setSelectedId, setStatus, setTheme, setShowNewEntryModal, setSearchHighlight,
  } = useVaultStore();

  const [query, setQuery]           = useState("");
  const [opts, setOpts]             = useState(DEFAULT_OPTS);
  const [showOpts, setShowOpts]     = useState(false);
  const [hits, setHits]             = useState<SearchHit[] | null>(null);

  const filterActive = query.trim() !== "";

  // Run search (debounced) on query/opts/entries change.
  useEffect(() => {
    if (!filterActive) {
      setHits(null);
      setSearchHighlight(null);
      return;
    }
    const handle = setTimeout(() => {
      const options: SearchOptions = { query: query.trim(), ...opts };
      searchEntries(options).then(setHits).catch(() => setHits([]));
    }, 200);
    return () => clearTimeout(handle);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, opts, entries]);

  // Clear highlight when query is cleared.
  useEffect(() => {
    if (!filterActive) setSearchHighlight(null);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filterActive]);

  async function handleLock() {
    await lockVault();
    setStatus("locked");
  }

  function cycleTheme() {
    const order: Theme[] = ["light", "dark", "sepia"];
    setTheme(order[(order.indexOf(theme) + 1) % order.length]);
  }

  function clearSearch() {
    setQuery("");
    setHits(null);
    setSearchHighlight(null);
  }

  function handleResultClick(hit: SearchHit) {
    setSelectedId(hit.entry.id);
    setSearchHighlight(query.trim() || null);
  }

  const themeLabel: Record<Theme, string> = { light: "☀️", dark: "🌙", sepia: "📜" };

  // What to display in the entry list.
  const displayed: Array<{ id: string; title: string; date: string; hit?: SearchHit }> =
    hits !== null
      ? hits.map((h) => ({
          id: h.entry.id,
          title: h.entry.title?.trim() || "Untitled",
          date: format(parseISO(h.entry.created_at), "MMMM d, yyyy"),
          hit: h,
        }))
      : entries.map((e) => ({
          id: e.id,
          title: e.title?.trim() || "Untitled",
          date: format(parseISO(e.created_at), "MMMM d, yyyy"),
        }));

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <span className="sidebar-title">Journal</span>
        <div className="sidebar-actions">
          <button className="icon-btn" title="Cycle theme" onClick={cycleTheme}>
            {themeLabel[theme]}
          </button>
          <button className="icon-btn" title="New entry" onClick={() => setShowNewEntryModal(true)}>
            ✏️
          </button>
          <button className="icon-btn" title="Lock" onClick={handleLock}>
            🔒
          </button>
        </div>
      </div>

      {/* Search bar + options toggle */}
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
            <button className="search-clear" title="Clear" onClick={clearSearch}>×</button>
          )}
          <button
            className={`search-opts-btn${showOpts ? " active" : ""}`}
            title="Search options"
            onClick={() => setShowOpts((v) => !v)}
            aria-label="Toggle search options"
          >
            ⚙
          </button>
        </div>

        {showOpts && (
          <div className="search-opts-panel">
            <div className="search-opts-row">
              <span className="search-opts-label">Search in</span>
              <div className="search-opts-checks">
                {(
                  [
                    ["in_body",     "Body"],
                    ["in_title",    "Title"],
                    ["in_tags",     "Tags"],
                    ["in_metadata", "Metadata"],
                  ] as const
                ).map(([key, label]) => (
                  <label key={key} className="search-opts-check">
                    <input
                      type="checkbox"
                      checked={opts[key]}
                      onChange={(e) => setOpts((o) => ({ ...o, [key]: e.target.checked }))}
                    />
                    {label}
                  </label>
                ))}
              </div>
            </div>

            <div className="search-opts-row">
              <span className="search-opts-label">Match</span>
              <div className="search-opts-radios">
                <label className="search-opts-radio">
                  <input
                    type="radio"
                    checked={opts.match_all_words}
                    onChange={() => setOpts((o) => ({ ...o, match_all_words: true }))}
                  />
                  All words
                </label>
                <label className="search-opts-radio">
                  <input
                    type="radio"
                    checked={!opts.match_all_words}
                    onChange={() => setOpts((o) => ({ ...o, match_all_words: false }))}
                  />
                  Exact phrase
                </label>
              </div>
            </div>

            <div className="search-opts-row">
              <span className="search-opts-label">Sort</span>
              <div className="search-opts-radios">
                <label className="search-opts-radio">
                  <input
                    type="radio"
                    checked={opts.sort_by_relevance}
                    onChange={() => setOpts((o) => ({ ...o, sort_by_relevance: true }))}
                  />
                  Relevance
                </label>
                <label className="search-opts-radio">
                  <input
                    type="radio"
                    checked={!opts.sort_by_relevance}
                    onChange={() => setOpts((o) => ({ ...o, sort_by_relevance: false }))}
                  />
                  Newest
                </label>
              </div>
            </div>
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
        {displayed.map(({ id, title, date, hit }) => (
          <button
            key={id}
            className={`entry-item${id === selectedId ? " selected" : ""}`}
            onClick={() => hit ? handleResultClick(hit) : setSelectedId(id)}
          >
            <span className="entry-label">{title}</span>
            <span className="entry-meta">{date}</span>
            {hit?.snippet && (
              <span className="entry-snippet">
                {hit.snippet.segments.map((seg, i) =>
                  seg.hit
                    ? <mark key={i} className="search-hit">{seg.text}</mark>
                    : <span key={i}>{seg.text}</span>
                )}
              </span>
            )}
          </button>
        ))}
      </div>
    </aside>
  );
}
