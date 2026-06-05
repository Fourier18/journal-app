import { useState, useRef, useMemo } from "react";
import { format, parseISO } from "date-fns";
import type { Entry, EntrySummary } from "../lib/commands";
import { useVaultStore } from "../store/vault";
import "./EntryHeader.css";

interface Props {
  entry: Entry;
  saveState: "saved" | "saving" | "unsaved";
  onEntryChange: (e: Entry) => void;
  onDelete: () => void;
}

export default function EntryHeader({ entry, saveState, onEntryChange, onDelete }: Props) {
  const [tagInput, setTagInput] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [showTagDropdown, setShowTagDropdown] = useState(false);
  const tagRef = useRef<HTMLInputElement>(null);

  const { entries } = useVaultStore();

  // Recent entries with tags (exclude current, newest first, max 6) for bundles.
  const tagBundles = useMemo(() =>
    entries.filter((e) => e.id !== entry.id && e.tags.length > 0).slice(0, 6),
  [entries, entry.id]);

  // All existing tags, filtered to those not yet on this entry and matching the input.
  const tagSuggestions = useMemo(() => {
    const set = new Set<string>();
    for (const e of entries) for (const t of e.tags) set.add(t);
    const q = tagInput.toLowerCase().trim();
    return Array.from(set)
      .filter((t) => !entry.tags.includes(t) && (q === "" || t.includes(q)))
      .sort();
  }, [entries, entry.tags, tagInput]);

  const isDaily = entry.entry_type === "daily";
  const displayDate = new Date(entry.created_at).toLocaleDateString(undefined, {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
  });

  function handleTitleChange(e: React.ChangeEvent<HTMLInputElement>) {
    onEntryChange({ ...entry, title: e.target.value || undefined });
  }

  function addTag(raw: string) {
    const tag = raw.trim().toLowerCase().replace(/\s+/g, "-");
    if (!tag || entry.tags.includes(tag)) return;
    onEntryChange({ ...entry, tags: [...entry.tags, tag] });
    setTagInput("");
  }

  function applyBundle(bundle: EntrySummary) {
    const merged = [...entry.tags];
    for (const t of bundle.tags) {
      if (!merged.includes(t)) merged.push(t);
    }
    onEntryChange({ ...entry, tags: merged });
    setShowTagDropdown(false);
    tagRef.current?.blur();
  }

  function removeTag(tag: string) {
    onEntryChange({ ...entry, tags: entry.tags.filter((t) => t !== tag) });
  }

  function handleTagKey(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      addTag(tagInput);
    } else if (e.key === "Backspace" && !tagInput && entry.tags.length > 0) {
      removeTag(entry.tags[entry.tags.length - 1]);
    }
  }

  return (
    <div className="entry-header">
      {/* Title row */}
      <div className="header-top">
        <div className="header-title-area">
          {isDaily ? (
            <span className="entry-date-label">{displayDate}</span>
          ) : (
            <input
              className="entry-title-input"
              type="text"
              value={entry.title ?? ""}
              onChange={handleTitleChange}
              placeholder="Untitled entry…"
            />
          )}
        </div>
        <div className="header-actions">
          <span className={`save-pill ${saveState}`}>
            {saveState === "saved" ? "Saved" : saveState === "saving" ? "Saving…" : "•"}
          </span>
          {confirmDelete ? (
            <span className="delete-confirm">
              Delete?&nbsp;
              <button className="del-yes" onClick={onDelete}>Yes</button>
              &nbsp;/&nbsp;
              <button className="del-no" onClick={() => setConfirmDelete(false)}>No</button>
            </span>
          ) : (
            <button
              className="icon-btn small"
              title="Delete entry"
              onClick={() => setConfirmDelete(true)}
            >
              🗑
            </button>
          )}
        </div>
      </div>

      {/* Tags row + suggestion dropdown */}
      <div className="tags-area">
        <div className="tags-row" onClick={() => tagRef.current?.focus()}>
          {entry.tags.map((tag) => (
            <span key={tag} className="tag-chip">
              {tag}
              <button
                className="tag-remove"
                onClick={(e) => { e.stopPropagation(); removeTag(tag); }}
                aria-label={`Remove tag ${tag}`}
              >
                ×
              </button>
            </span>
          ))}
          <input
            ref={tagRef}
            className="tag-input"
            value={tagInput}
            onChange={(e) => setTagInput(e.target.value)}
            onKeyDown={handleTagKey}
            onFocus={() => setShowTagDropdown(true)}
            onBlur={() => setTimeout(() => setShowTagDropdown(false), 150)}
            placeholder={entry.tags.length === 0 ? "Add tags…" : ""}
          />
        </div>

        {showTagDropdown && (tagBundles.length > 0 || tagSuggestions.length > 0) && (
          <div className="tag-dropdown">
            {tagBundles.length > 0 && (
              <div className="tag-dropdown-group">
                <div className="tag-dropdown-group-label">From entries</div>
                {tagBundles.map((e) => {
                  const label = e.title || format(parseISO(e.created_at), "MMM d, yyyy");
                  return (
                    <button
                      key={e.id}
                      className="tag-dropdown-bundle"
                      onMouseDown={(ev) => { ev.preventDefault(); applyBundle(e); }}
                      title={e.tags.join(", ")}
                    >
                      <span className="tag-dropdown-bundle-label">{label}</span>
                      <span className="tag-dropdown-bundle-count">{e.tags.length} tags</span>
                    </button>
                  );
                })}
              </div>
            )}
            {tagSuggestions.length > 0 && (
              <div className="tag-dropdown-group">
                <div className="tag-dropdown-group-label">Existing tags</div>
                <div className="tag-suggestions-list">
                  {tagSuggestions.map((t) => (
                    <button
                      key={t}
                      className="tag-suggestion-chip"
                      onMouseDown={(ev) => { ev.preventDefault(); addTag(t); }}
                    >
                      #{t}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
