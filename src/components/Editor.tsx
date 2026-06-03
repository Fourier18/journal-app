import { useEffect, useRef, useState, useCallback } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { oneDark } from "@codemirror/theme-one-dark";
import { EditorView } from "@codemirror/view";
import { readEntry, updateEntry } from "../lib/commands";
import { useVaultStore } from "../store/vault";
import type { Entry } from "../lib/commands";
import "./Editor.css";

const AUTOSAVE_MS = 1500;

const sepiaTheme = EditorView.theme({
  "&": { background: "var(--editor-bg)", color: "var(--editor-text)" },
  ".cm-content": { fontFamily: "var(--font-editor)", fontSize: "17px", lineHeight: "1.7" },
  ".cm-cursor": { borderLeftColor: "var(--accent)" },
  ".cm-selectionBackground": { background: "var(--bg-selected) !important" },
  ".cm-gutters": { display: "none" },
  ".cm-focused .cm-cursor": { borderLeftColor: "var(--accent)" },
});

const lightTheme = EditorView.theme({
  "&": { background: "var(--editor-bg)", color: "var(--editor-text)" },
  ".cm-content": { fontFamily: "var(--font-editor)", fontSize: "17px", lineHeight: "1.7" },
  ".cm-cursor": { borderLeftColor: "var(--accent)" },
  ".cm-gutters": { display: "none" },
});

export default function Editor() {
  const { selectedId, theme } = useVaultStore();
  const [entry, setEntry] = useState<Entry | null>(null);
  const [body, setBody] = useState("");
  const [saveState, setSaveState] = useState<"saved" | "saving" | "unsaved">("saved");
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load entry when selection changes
  useEffect(() => {
    if (!selectedId) {
      setEntry(null);
      setBody("");
      return;
    }
    readEntry(selectedId).then(({ entry, body }) => {
      setEntry(entry);
      setBody(body);
      setSaveState("saved");
    });
  }, [selectedId]);

  const save = useCallback(
    async (currentEntry: Entry, currentBody: string) => {
      setSaveState("saving");
      const updated: Entry = { ...currentEntry, updated_at: new Date().toISOString() };
      await updateEntry(updated.id, updated, currentBody);
      setEntry(updated);
      setSaveState("saved");
    },
    []
  );

  function handleChange(value: string) {
    setBody(value);
    setSaveState("unsaved");
    if (saveTimer.current) clearTimeout(saveTimer.current);
    if (entry) {
      saveTimer.current = setTimeout(() => save(entry, value), AUTOSAVE_MS);
    }
  }

  // Cleanup timer on unmount
  useEffect(() => () => { if (saveTimer.current) clearTimeout(saveTimer.current); }, []);

  const cmTheme = theme === "dark" ? oneDark : theme === "sepia" ? sepiaTheme : lightTheme;

  if (!selectedId) {
    return (
      <div className="editor-empty">
        <p>Select an entry from the sidebar, or click ✏️ to create one.</p>
      </div>
    );
  }

  if (!entry) {
    return <div className="editor-empty"><p>Loading…</p></div>;
  }

  return (
    <div className="editor-pane">
      <div className="editor-statusbar">
        <span className="editor-date">
          {new Date(entry.created_at).toLocaleDateString(undefined, {
            weekday: "long",
            year: "numeric",
            month: "long",
            day: "numeric",
          })}
        </span>
        <span className={`save-indicator ${saveState}`}>
          {saveState === "saved" ? "Saved" : saveState === "saving" ? "Saving…" : "Unsaved"}
        </span>
      </div>
      <div className="editor-scroll">
        <CodeMirror
          value={body}
          extensions={[markdown(), EditorView.lineWrapping]}
          theme={cmTheme}
          onChange={handleChange}
          basicSetup={{
            lineNumbers: false,
            foldGutter: false,
            dropCursor: true,
            allowMultipleSelections: true,
            indentOnInput: true,
          }}
          className="cm-editor-wrap"
        />
      </div>
    </div>
  );
}
