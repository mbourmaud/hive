// ── Fuzzy match (case-insensitive substring) ────────────────────────────────

export function fuzzyMatch(query: string, text: string): boolean {
  if (!query) return true;
  return text.toLowerCase().includes(query.toLowerCase());
}

// ── Shortcut display ────────────────────────────────────────────────────────

export function ShortcutKeys({ keys }: { keys: string[] }) {
  return (
    <span data-slot="cmd-item-shortcut">
      {keys.map((k) => (
        <kbd key={k} data-slot="cmd-kbd">
          {k}
        </kbd>
      ))}
    </span>
  );
}
