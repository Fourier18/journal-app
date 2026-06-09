import {
  Decoration,
  DecorationSet,
  EditorView,
  MatchDecorator,
  ViewPlugin,
  ViewUpdate,
} from "@codemirror/view";

const highlightMark = Decoration.mark({ class: "cm-search-highlight" });

/** Theme injection — keeps styles co-located with the extension. */
const highlightTheme = EditorView.baseTheme({
  ".cm-search-highlight": {
    background: "var(--search-highlight-bg)",
    borderRadius: "2px",
  },
});

/**
 * CodeMirror 6 extension that highlights every occurrence of `term` in the
 * editor and scrolls the first match into view on mount or term change.
 *
 * Modeled on the MatchDecorator pattern used in wikilinkExtension.ts.
 * Pass a fresh extension (via useMemo) whenever `term` changes.
 */
export function searchHighlightExtension(term: string | null) {
  if (!term || !term.trim()) return [];

  const escaped = term.trim().replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const regexp = new RegExp(escaped, "gi");

  const decorator = new MatchDecorator({
    regexp,
    decoration: () => highlightMark,
  });

  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      scrolled = false;

      constructor(view: EditorView) {
        this.decorations = decorator.createDeco(view);
        this.scrollFirstIntoView(view);
      }

      update(update: ViewUpdate) {
        this.decorations = decorator.updateDeco(update, this.decorations);
        if (!this.scrolled && this.decorations.size > 0) {
          this.scrollFirstIntoView(update.view);
        }
      }

      scrollFirstIntoView(view: EditorView) {
        // Iterate to the first decoration and scroll it into view.
        const iter = this.decorations.iter();
        iter.next();
        if (iter.value) {
          view.dispatch({
            effects: EditorView.scrollIntoView(iter.from, { y: "center" }),
          });
          this.scrolled = true;
        }
      }
    },
    { decorations: (v) => v.decorations }
  );

  return [highlightTheme, plugin];
}
