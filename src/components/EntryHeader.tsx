import { useState, useRef } from "react";
import type { Entry } from "../lib/commands";
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
  const tagRef = useRef<HTMLInputElement>(null);

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

      {/* Tags row */}
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
          onBlur={() => { if (tagInput) addTag(tagInput); }}
          placeholder={entry.tags.length === 0 ? "Add tags…" : ""}
        />
      </div>
    </div>
  );
}
