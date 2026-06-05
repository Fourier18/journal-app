import {
  Decoration,
  DecorationSet,
  EditorView,
  MatchDecorator,
  ViewPlugin,
  ViewUpdate,
  WidgetType,
} from "@codemirror/view";
import {
  autocompletion,
  CompletionContext,
  CompletionResult,
} from "@codemirror/autocomplete";
import { format, parseISO } from "date-fns";
import type { EntrySummary } from "./commands";

function entryLabel(e: EntrySummary): string {
  return e.title || format(parseISO(e.created_at), "MMMM d, yyyy");
}

class WikilinkWidget extends WidgetType {
  constructor(
    readonly id: string,
    readonly label: string,
    readonly broken: boolean,
    readonly navigate: (id: string) => void
  ) {
    super();
  }

  eq(other: WikilinkWidget) {
    return (
      other.id === this.id &&
      other.label === this.label &&
      other.broken === this.broken
    );
  }

  toDOM(): HTMLElement {
    const span = document.createElement("span");
    span.className = this.broken
      ? "cm-wikilink cm-wikilink-broken"
      : "cm-wikilink";
    span.textContent = this.label;
    span.title = this.broken
      ? `Broken link (entry not found: ${this.id})`
      : `→ ${this.label}`;
    span.addEventListener("mousedown", (e) => {
      if (!this.broken) {
        e.preventDefault();
        this.navigate(this.id);
      }
    });
    return span;
  }

  // Let mouse events propagate so click navigation works.
  ignoreEvent() {
    return false;
  }
}

export function wikilinkExtension(
  entries: EntrySummary[],
  navigate: (id: string) => void
) {
  const entryMap = new Map(entries.map((e) => [e.id, e]));

  // ── Autocomplete ────────────────────────────────────────────────────────────
  // Triggers when the user types [[, shows a searchable list of entries,
  // and inserts [[uuid]] on selection.
  const completionSource = (ctx: CompletionContext): CompletionResult | null => {
    const before = ctx.matchBefore(/\[\[[^\]\[]{0,80}/);
    if (!before) return null;

    const typed = before.text.slice(2).toLowerCase(); // text after [[
    const options = entries
      .filter((e) => entryLabel(e).toLowerCase().includes(typed))
      .slice(0, 20)
      .map((e) => ({
        label: entryLabel(e),
        detail: format(parseISO(e.created_at), "MMM d, yyyy"),
        // Use a function so we can swallow any ]] that closeBrackets auto-inserted.
        apply: (view: EditorView, _c: unknown, from: number, to: number) => {
          const after = view.state.doc.sliceString(to, to + 2);
          view.dispatch({
            changes: { from, to: after === "]]" ? to + 2 : to, insert: `[[${e.id}]]` },
            selection: { anchor: from + e.id.length + 4 },
          });
        },
      }));

    // If user has typed a filter but nothing matches, hide the popup.
    if (options.length === 0 && typed.length > 0) return null;

    return { from: before.from, options, filter: false };
  };

  // ── Decoration ──────────────────────────────────────────────────────────────
  // Replaces [[uuid]] in the editor with a clickable rendered-title widget.
  const decorator = new MatchDecorator({
    regexp: /\[\[([^\]]+)\]\]/g,
    decoration(match) {
      const id = match[1];
      const entry = entryMap.get(id);
      const label = entry ? entryLabel(entry) : id;
      const broken = !entry;
      return Decoration.replace({
        widget: new WikilinkWidget(id, label, broken, navigate),
        inclusive: false,
      });
    },
  });

  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(view: EditorView) {
        this.decorations = decorator.createDeco(view);
      }
      update(update: ViewUpdate) {
        this.decorations = decorator.updateDeco(update, this.decorations);
      }
    },
    {
      decorations: (v) => v.decorations,
      // Make each [[uuid]] widget a single atomic unit (cursor can't enter it).
      provide: (p) =>
        EditorView.atomicRanges.of((view) => {
          return view.plugin(p)?.decorations ?? Decoration.none;
        }),
    }
  );

  return [autocompletion({ override: [completionSource] }), plugin];
}
