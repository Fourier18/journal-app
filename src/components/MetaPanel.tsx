import { useState, useRef } from "react";
import type { Entry, MetadataValue } from "../lib/commands";
import "./MetaPanel.css";

interface Props {
  entry: Entry;
  onEntryChange: (e: Entry) => void;
}

// Built-in fields with known semantics. Only these two are "special" — everything
// else the user adds is either a plain number or plain text.
const KNOWN_FIELDS: Record<string, { label: string; unit?: string }> = {
  mood:  { label: "Mood",  unit: "/ 10" },
  sleep: { label: "Sleep", unit: "hrs"  },
};

export default function MetaPanel({ entry, onEntryChange }: Props) {
  const [open, setOpen] = useState(true);
  const [newKey, setNewKey] = useState("");
  const [newIsNumber, setNewIsNumber] = useState(false);

  // Which field is currently being edited, and the raw string the user is typing.
  const [editingField, setEditingField] = useState<string | null>(null);
  const [editingValue, setEditingValue] = useState("");
  const editInputRef = useRef<HTMLInputElement>(null);

  const keys = Object.keys(entry.metadata);
  if (keys.length === 0 && !open) return null;

  function setField(key: string, value: MetadataValue) {
    onEntryChange({ ...entry, metadata: { ...entry.metadata, [key]: value } });
  }

  function removeField(key: string) {
    if (editingField === key) setEditingField(null);
    const next = { ...entry.metadata };
    delete next[key];
    onEntryChange({ ...entry, metadata: next });
  }

  function addField() {
    const k = newKey.trim().toLowerCase().replace(/\s+/g, "_");
    if (!k || k in entry.metadata) return;
    setField(k, newIsNumber ? 0 : "");
    setNewKey("");
    setNewIsNumber(false);
  }

  // ── Click-to-edit handlers ─────────────────────────────────────────────────

  function startEdit(key: string) {
    const val = entry.metadata[key];
    setEditingField(key);
    setEditingValue(typeof val === "number" ? String(val) : (val as string));
    // autoFocus via ref so the input is ready without a second click
    setTimeout(() => editInputRef.current?.select(), 0);
  }

  function commitEdit() {
    if (!editingField) return;
    const val = entry.metadata[editingField];
    const raw = editingValue.trim();

    if (typeof val === "number") {
      // Numeric field: only update if it parsed; otherwise keep the original value.
      const n = parseFloat(raw);
      if (!isNaN(n)) setField(editingField, n);
    } else {
      setField(editingField, raw);
    }
    setEditingField(null);
  }

  function cancelEdit() {
    setEditingField(null);
  }

  // ── Rendering helpers ──────────────────────────────────────────────────────

  function staticDisplay(key: string): string {
    const val = entry.metadata[key];
    const spec = KNOWN_FIELDS[key];
    if (typeof val === "number") {
      return spec?.unit ? `${val} ${spec.unit}` : String(val);
    }
    return val ? String(val) : "—";
  }

  return (
    <div className="meta-panel">
      <button className="meta-toggle" onClick={() => setOpen((o) => !o)}>
        <span className="meta-toggle-label">Metadata</span>
        <span className="meta-toggle-arrow">{open ? "▲" : "▼"}</span>
      </button>

      {open && (
        <div className="meta-fields">
          {keys.map((key) => {
            const spec = KNOWN_FIELDS[key];
            const isEditing = editingField === key;
            const val = entry.metadata[key];
            const isNumeric = typeof val === "number";

            return (
              <div
                key={key}
                className={`meta-row${isEditing ? " meta-row-editing" : ""}`}
              >
                <span className="meta-key">{spec?.label ?? key}</span>

                {isEditing ? (
                  <div className="meta-edit-wrap">
                    <input
                      ref={editInputRef}
                      autoFocus
                      className={isNumeric ? "meta-num" : "meta-text"}
                      // type="text" avoids browser blocking intermediate numeric states
                      type="text"
                      inputMode={isNumeric ? "decimal" : "text"}
                      value={editingValue}
                      onChange={(e) => setEditingValue(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") { e.preventDefault(); commitEdit(); }
                        if (e.key === "Escape") cancelEdit();
                      }}
                      onBlur={commitEdit}
                    />
                    {spec?.unit && <span className="meta-unit">{spec.unit}</span>}
                  </div>
                ) : (
                  <button
                    className="meta-value-btn"
                    onClick={() => startEdit(key)}
                    title="Click to edit"
                  >
                    {staticDisplay(key)}
                  </button>
                )}

                <button
                  className="meta-remove"
                  onClick={(e) => { e.stopPropagation(); removeField(key); }}
                  title={`Remove ${key}`}
                >
                  ×
                </button>
              </div>
            );
          })}

          {/* Add field row */}
          <div className="meta-add-row">
            <input
              className="meta-add-input"
              type="text"
              value={newKey}
              onChange={(e) => setNewKey(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addField()}
              placeholder="Add field…"
            />
            <button
              className={`meta-type-toggle${newIsNumber ? " is-number" : ""}`}
              onClick={() => setNewIsNumber((v) => !v)}
              title={newIsNumber ? "Number field — click to switch to text" : "Text field — click to switch to number"}
            >
              {newIsNumber ? "123" : "abc"}
            </button>
            <button className="meta-add-btn" onClick={addField} disabled={!newKey.trim()}>
              +
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
