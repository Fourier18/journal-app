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

  // Every existing tag (across all entries), filtered by what's typed. Applied
  // tags are shown too — marked as selected — so the dropdown is a full picker.
  const allTags = useMemo(() => {
    const set = new Set<string>();
    for (const e of entries) for (const t of e.tags) set.add(t);
    const q = tagInput.toLowerCase().trim();
    return Array.from(set)
      .filter((t) => q === "" || t.includes(q))
      .sort();
  }, [entries, tagInput]);

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
    // Keep the dropdown open so several bundles / tags can be picked in a row.
  }

  function removeTag(tag: string) {
    onEntryChange({ ...entry, tags: entry.tags.filter((t) => t !== tag) });
  }

  // Toggle an existing tag from the dropdown picker (doesn't clear the filter input).
  function toggleExistingTag(tag: string) {
    if (entry.tags.includes(tag)) {
      onEntryChange({ ...entry, tags: entry.tags.filter((t) => t !== tag) });
    } else {
      onEntryChange({ ...entry, tags: [...entry.tags, tag] });
    }
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
          <input
            className="entry-title-input"
            type="text"
            value={entry.title ?? ""}
            onChange={handleTitleChange}
            placeholder="Untitled entry…"
          />
          <span className="entry-date-sub">{displayDate}</span>
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

        {showTagDropdown && (tagBundles.length > 0 || allTags.length > 0) && (
          <div className="tag-dropdown">
            {tagBundles.length > 0 && (
              <div className="tag-dropdown-group">
                <div className="tag-dropdown-group-label">Copy tags from entry</div>
                {tagBundles.map((e) => {
                  const label = e.title?.trim() || format(parseISO(e.created_at), "MMM d, yyyy");
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
            {allTags.length > 0 && (
              <div className="tag-dropdown-group">
                <div className="tag-dropdown-group-label">Tags</div>
                <div className="tag-suggestions-list">
                  {allTags.map((t) => {
                    const applied = entry.tags.includes(t);
                    return (
                      <button
                        key={t}
                        className={`tag-suggestion-chip${applied ? " applied" : ""}`}
                        onMouseDown={(ev) => { ev.preventDefault(); toggleExistingTag(t); }}
                      >
                        {applied ? "✓ " : "#"}{t}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
