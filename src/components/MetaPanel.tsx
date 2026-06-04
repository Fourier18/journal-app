import { useState } from "react";
import type { Entry, MetadataValue } from "../lib/commands";
import "./MetaPanel.css";

interface Props {
  entry: Entry;
  onEntryChange: (e: Entry) => void;
}

// Fields with special UI treatment
const KNOWN_FIELDS: Record<string, { label: string; type: "number"; min?: number; max?: number; unit?: string }> = {
  mood:  { label: "Mood",  type: "number", min: 1, max: 10, unit: "/ 10" },
  sleep: { label: "Sleep", type: "number", min: 0, max: 24,  unit: "hrs"  },
};

export default function MetaPanel({ entry, onEntryChange }: Props) {
  const [open, setOpen] = useState(true);
  const [newKey, setNewKey] = useState("");

  const keys = Object.keys(entry.metadata);
  if (keys.length === 0 && !open) return null; // hide entirely when empty + collapsed

  function setField(key: string, value: MetadataValue) {
    onEntryChange({ ...entry, metadata: { ...entry.metadata, [key]: value } });
  }

  function removeField(key: string) {
    const next = { ...entry.metadata };
    delete next[key];
    onEntryChange({ ...entry, metadata: next });
  }

  function addField() {
    const k = newKey.trim().toLowerCase().replace(/\s+/g, "_");
    if (!k || k in entry.metadata) return;
    setField(k, "");
    setNewKey("");
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
            const val = entry.metadata[key];
            return (
              <div key={key} className="meta-row">
                <label className="meta-key">{spec?.label ?? key}</label>
                {spec ? (
                  <div className="meta-num-wrap">
                    <input
                      type="number"
                      className="meta-num"
                      value={typeof val === "number" ? val : ""}
                      min={spec.min}
                      max={spec.max}
                      step={1}
                      onChange={(e) => setField(key, parseFloat(e.target.value) || 0)}
                    />
                    {spec.unit && <span className="meta-unit">{spec.unit}</span>}
                  </div>
                ) : (
                  <input
                    type="text"
                    className="meta-text"
                    value={typeof val === "string" ? val : String(val)}
                    onChange={(e) => setField(key, e.target.value)}
                    placeholder="—"
                  />
                )}
                <button
                  className="meta-remove"
                  onClick={() => removeField(key)}
                  title={`Remove ${key}`}
                >
                  ×
                </button>
              </div>
            );
          })}

          {/* Add field */}
          <div className="meta-add-row">
            <input
              className="meta-add-input"
              type="text"
              value={newKey}
              onChange={(e) => setNewKey(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addField()}
              placeholder="Add field…"
            />
            <button className="meta-add-btn" onClick={addField} disabled={!newKey.trim()}>
              +
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
