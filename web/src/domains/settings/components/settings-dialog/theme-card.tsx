import { Trash2 } from "lucide-react";
import { useCallback } from "react";
import type { ThemeName } from "@/shared/theme/use-theme";

// ── Built-in theme card ─────────────────────────────────────────────────────

export function ThemeCard({
  label,
  accent,
  bg,
  isActive,
  onSelect,
}: {
  name: ThemeName;
  label: string;
  accent: string;
  bg: string;
  isActive: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      data-slot="settings-theme-card"
      data-active={isActive || undefined}
      onClick={onSelect}
      aria-label={`Select ${label} theme`}
    >
      <div data-slot="settings-theme-swatch" style={{ background: bg }}>
        <div data-slot="settings-theme-accent" style={{ background: accent }} />
      </div>
      <span data-slot="settings-theme-label">{label}</span>
    </button>
  );
}

// ── Custom theme card with delete button ────────────────────────────────────

export function CustomThemeCard({
  label,
  accent,
  bg,
  isActive,
  onSelect,
  onDelete,
}: {
  id: string;
  label: string;
  accent: string;
  bg: string;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
}) {
  const handleDelete = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onDelete();
    },
    [onDelete],
  );

  return (
    <button
      type="button"
      data-slot="settings-theme-card"
      data-custom
      data-active={isActive || undefined}
      onClick={onSelect}
      aria-label={`Select ${label} custom theme`}
    >
      <div data-slot="settings-theme-swatch" style={{ background: bg }}>
        <div data-slot="settings-theme-accent" style={{ background: accent }} />
      </div>
      <span data-slot="settings-theme-label">{label}</span>
      <button
        type="button"
        data-slot="settings-theme-delete"
        onClick={handleDelete}
        aria-label={`Delete ${label} theme`}
      >
        <Trash2 className="h-3 w-3" />
      </button>
    </button>
  );
}
