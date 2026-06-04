import { useState } from "react";
import { createEntry, listEntries, type Entry, type EntryType } from "../lib/commands";
import { TEMPLATES, type JournalTemplate } from "../lib/templates";
import { useVaultStore } from "../store/vault";
import "./NewEntryModal.css";

export default function NewEntryModal() {
  const { setShowNewEntryModal, setEntries, setSelectedId } = useVaultStore();
  const [entryType, setEntryType] = useState<EntryType>("daily");
  const [template, setTemplate] = useState<JournalTemplate>(TEMPLATES[0]);
  const [title, setTitle] = useState("");
  const [busy, setBusy] = useState(false);

  async function handleCreate() {
    setBusy(true);
    const now = new Date().toISOString();
    const entry: Entry = {
      id: crypto.randomUUID(),
      created_at: now,
      updated_at: now,
      entry_type: entryType,
      template: template.id,
      tags: [],
      metadata: { ...template.defaultMetadata },
      title: entryType === "free_form" && title.trim() ? title.trim() : undefined,
    };
    await createEntry(entry, template.body);
    const fresh = await listEntries();
    setEntries(fresh);
    setSelectedId(entry.id);
    setShowNewEntryModal(false);
  }

  return (
    <div className="modal-overlay" onClick={() => setShowNewEntryModal(false)}>
      <div className="modal-card" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span className="modal-title">New Entry</span>
          <button className="modal-close" onClick={() => setShowNewEntryModal(false)}>
            ×
          </button>
        </div>

        {/* Entry type */}
        <section className="modal-section">
          <label className="modal-label">Type</label>
          <div className="type-toggle">
            <button
              className={`type-btn${entryType === "daily" ? " active" : ""}`}
              onClick={() => setEntryType("daily")}
            >
              📅 Daily
            </button>
            <button
              className={`type-btn${entryType === "free_form" ? " active" : ""}`}
              onClick={() => setEntryType("free_form")}
            >
              📝 Free-form
            </button>
          </div>
        </section>

        {/* Title (free-form only) */}
        {entryType === "free_form" && (
          <section className="modal-section">
            <label className="modal-label">Title (optional)</label>
            <input
              className="modal-input"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="My entry title…"
              autoFocus
            />
          </section>
        )}

        {/* Template picker */}
        <section className="modal-section">
          <label className="modal-label">Template</label>
          <div className="template-grid">
            {TEMPLATES.map((t) => (
              <button
                key={t.id}
                className={`template-card${template.id === t.id ? " active" : ""}`}
                onClick={() => setTemplate(t)}
              >
                <span className="template-name">{t.name}</span>
                <span className="template-desc">{t.description}</span>
              </button>
            ))}
          </div>
        </section>

        <div className="modal-footer">
          <button
            className="modal-btn secondary"
            onClick={() => setShowNewEntryModal(false)}
          >
            Cancel
          </button>
          <button className="modal-btn primary" onClick={handleCreate} disabled={busy}>
            {busy ? "Creating…" : "Create Entry"}
          </button>
        </div>
      </div>
    </div>
  );
}
