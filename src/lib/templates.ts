import type { MetadataValue } from "./commands";

export interface JournalTemplate {
  id: string;
  name: string;
  description: string;
  body: string;
  defaultMetadata: Record<string, MetadataValue>;
}

export const TEMPLATES: JournalTemplate[] = [
  {
    id: "blank",
    name: "Blank",
    description: "Start with a clean page",
    body: "",
    defaultMetadata: {},
  },
  {
    id: "gratitude",
    name: "Gratitude",
    description: "Three things I'm grateful for",
    body: `## Gratitude

1.
2.
3.

## One good thing that happened today



## How I'm feeling

`,
    defaultMetadata: { mood: 7 },
  },
  {
    id: "dream_log",
    name: "Dream Log",
    description: "Capture last night's dreams",
    body: `## The dream

(Describe it in as much detail as you remember)

## Feelings & impressions



## Recurring symbols or themes


`,
    defaultMetadata: { sleep: 7 },
  },
  {
    id: "daily_standup",
    name: "Daily Standup",
    description: "Work log: yesterday, today, blockers",
    body: `## Yesterday

-

## Today

-

## Blockers

-

## Notes

`,
    defaultMetadata: {},
  },
  {
    id: "mood_check_in",
    name: "Mood Check-in",
    description: "Track how you're feeling and why",
    body: `## How I'm feeling right now



## Why I think I feel this way



## One thing I can do for myself today


`,
    defaultMetadata: { mood: 5, weather: "" },
  },
  {
    id: "work_log",
    name: "Work Log",
    description: "Tasks, progress, and plans",
    body: `## Completed

-

## In progress

-

## Planned for tomorrow

-

## Notes

`,
    defaultMetadata: {},
  },
];

export function getTemplate(id: string): JournalTemplate {
  return TEMPLATES.find((t) => t.id === id) ?? TEMPLATES[0];
}
