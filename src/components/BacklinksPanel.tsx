import { useEffect, useState } from "react";
import { format, parseISO } from "date-fns";
import { getBacklinks, type EntrySummary } from "../lib/commands";
import { useVaultStore } from "../store/vault";
import "./BacklinksPanel.css";

interface Props {
  entryId: string;
}

export default function BacklinksPanel({ entryId }: Props) {
  const setSelectedId = useVaultStore((s) => s.setSelectedId);
  const [backlinks, setBacklinks] = useState<EntrySummary[]>([]);

  useEffect(() => {
    getBacklinks(entryId)
      .then(setBacklinks)
      .catch(() => setBacklinks([]));
  }, [entryId]);

  if (backlinks.length === 0) return null;

  return (
    <div className="backlinks-panel">
      <div className="backlinks-header">
        Linked from ({backlinks.length})
      </div>
      <ul className="backlinks-list">
        {backlinks.map((e) => {
          const label = e.title || format(parseISO(e.created_at), "MMMM d, yyyy");
          const sub = format(parseISO(e.created_at), "MMM d, yyyy");
          return (
            <li key={e.id}>
              <button
                className="backlink-item"
                onClick={() => setSelectedId(e.id)}
                title={`Open "${label}"`}
              >
                <span className="backlink-title">{label}</span>
                <span className="backlink-date">{sub}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
