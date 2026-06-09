import { useEffect, useRef, useState, useCallback, useMemo } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { oneDark } from "@codemirror/theme-one-dark";
import { EditorView } from "@codemirror/view";
import { readEntry, updateEntry, deleteEntry, listEntries } from "../lib/commands";
import type { Entry } from "../lib/commands";
import { useVaultStore } from "../store/vault";
import { wikilinkExtension } from "../lib/wikilinkExtension";
import { searchHighlightExtension } from "../lib/searchHighlightExtension";
import EntryHeader from "./EntryHeader";
import MetaPanel from "./MetaPanel";
import BacklinksPanel from "./BacklinksPanel";
import "./Editor.css";

const AUTOSAVE_MS = 1500;

const sepiaTheme = EditorView.theme({
  "&": { background: "var(--editor-bg)", color: "var(--editor-text)" },
  ".cm-content": { fontFamily: "var(--font-editor)", fontSize: "17px", lineHeight: "1.7" },
  ".cm-cursor": { borderLeftColor: "var(--accent)" },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground":
    { background: "var(--bg-selected) !important" },
  ".cm-gutters": { display: "none" },
});

const lightTheme = EditorView.theme({
  "&": { background: "var(--editor-bg)", color: "var(--editor-text)" },
  ".cm-content": { fontFamily: "var(--font-editor)", fontSize: "17px", lineHeight: "1.7" },
  ".cm-cursor": { borderLeftColor: "var(--accent)" },
  ".cm-gutters": { display: "none" },
});

export default function Editor() {
  const { selectedId, theme, entries, searchHighlight, setSelectedId, setEntries, patchEntry } = useVaultStore();
  const [entry, setEntry] = useState<Entry | null>(null);
  const [body, setBody] = useState("");
  const [saveState, setSaveState] = useState<"saved" | "saving" | "unsaved">("saved");

  // Refs so callbacks always see fresh values without stale closures
  const entryRef = useRef<Entry | null>(null);
  const bodyRef  = useRef("");
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  entryRef.current = entry;
  bodyRef.current  = body;

  const flushSave = useCallback(async (e: Entry, b: string) => {
    setSaveState("saving");
    const updated: Entry = { ...e, updated_at: new Date().toISOString() };
    await updateEntry(updated.id, updated, b);
    setEntry(updated);
    entryRef.current = updated;
    setSaveState("saved");
  }, []);

  const scheduleSave = useCallback((e: Entry, b: string) => {
    setSaveState("unsaved");
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => flushSave(e, b), AUTOSAVE_MS);
  }, [flushSave]);

  // Load entry when selection changes
  useEffect(() => {
    if (!selectedId) { setEntry(null); setBody(""); return; }
    // Flush any pending save for the previous entry first
    if (saveTimer.current) { clearTimeout(saveTimer.current); saveTimer.current = null; }
    if (entryRef.current && saveState === "unsaved") {
      flushSave(entryRef.current, bodyRef.current);
    }
    readEntry(selectedId).then(({ entry: e, body: b }) => {
      setEntry(e); setBody(b); setSaveState("saved");
    });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedId]);

  // Cleanup on unmount
  useEffect(() => () => { if (saveTimer.current) clearTimeout(saveTimer.current); }, []);

  function handleBodyChange(value: string) {
    setBody(value);
    if (entryRef.current) scheduleSave(entryRef.current, value);
  }

  function handleEntryChange(updated: Entry) {
    setEntry(updated);
    // Keep the sidebar list in sync immediately (title, tags) without waiting
    // for the debounced save to land.
    patchEntry(updated.id, { title: updated.title, tags: updated.tags });
    scheduleSave(updated, bodyRef.current);
  }

  async function handleDelete() {
    if (!entry) return;
    if (saveTimer.current) clearTimeout(saveTimer.current);
    await deleteEntry(entry.id);
    const fresh = await listEntries();
    setEntries(fresh);
    setSelectedId(null);
  }

  // Rebuild wikilink extension whenever the entries list changes so that
  // autocomplete and rendered titles stay current.
  const wikilinkExts = useMemo(
    () => wikilinkExtension(entries, setSelectedId),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [entries]
  );

  // Rebuild highlight extension whenever the active search term changes.
  const highlightExts = useMemo(
    () => searchHighlightExtension(searchHighlight),
    [searchHighlight]
  );

  const cmTheme = theme === "dark" ? oneDark : theme === "sepia" ? sepiaTheme : lightTheme;

  if (!selectedId) {
    return (
      <div className="editor-empty">
        <p>Select an entry, or click ✏️ to create one.</p>
      </div>
    );
  }

  if (!entry) {
    return <div className="editor-empty"><p>Loading…</p></div>;
  }

  return (
    <div className="editor-pane">
      <EntryHeader
        entry={entry}
        saveState={saveState}
        onEntryChange={handleEntryChange}
        onDelete={handleDelete}
      />
      <MetaPanel entry={entry} onEntryChange={handleEntryChange} />
      <div className="editor-scroll">
        <CodeMirror
          value={body}
          extensions={[markdown(), EditorView.lineWrapping, ...wikilinkExts, ...highlightExts]}
          theme={cmTheme}
          onChange={handleBodyChange}
          basicSetup={{
            lineNumbers: false,
            foldGutter: false,
            dropCursor: true,
            allowMultipleSelections: true,
            indentOnInput: true,
            autocompletion: false,
          }}
          className="cm-editor-wrap"
        />
      </div>
      <BacklinksPanel entryId={selectedId} />
    </div>
  );
}
