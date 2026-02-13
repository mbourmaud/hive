import * as Dialog from "@radix-ui/react-dialog";
import * as Tabs from "@radix-ui/react-tabs";
import { Minus, Plus, Settings, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { cn } from "@/shared/lib/utils";
import type { ThemeName } from "@/shared/theme/use-theme";
import { THEMES, useTheme } from "@/shared/theme/use-theme";
import "./settings-dialog.css";

// ── Types ────────────────────────────────────────────────────────────────────

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  models?: { id: string; name: string }[];
  selectedModel?: string;
  onModelChange?: (modelId: string) => void;
}

interface AppSettings {
  fontSize: number;
}

// ── localStorage helpers ────────────────────────────────────────────────────

const SETTINGS_KEY = "hive-settings";
const VERSION = "0.1.0";
const DEFAULT_FONT_SIZE = 14;
const MIN_FONT_SIZE = 12;
const MAX_FONT_SIZE = 18;

function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return { fontSize: DEFAULT_FONT_SIZE };
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const fontSize = typeof parsed.fontSize === "number" ? parsed.fontSize : DEFAULT_FONT_SIZE;
    return { fontSize: Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, fontSize)) };
  } catch {
    return { fontSize: DEFAULT_FONT_SIZE };
  }
}

function saveSettings(settings: AppSettings): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // quota exceeded
  }
}

// ── Component ────────────────────────────────────────────────────────────────

export function SettingsDialog({
  open,
  onOpenChange,
  models,
  selectedModel,
  onModelChange,
}: SettingsDialogProps) {
  const { theme, toggleTheme, themeName, setThemeName } = useTheme();
  const [settings, setSettings] = useState<AppSettings>(loadSettings);

  // Apply font size to document
  useEffect(() => {
    document.body.style.fontSize = `${settings.fontSize}px`;
    saveSettings(settings);
  }, [settings]);

  const adjustFontSize = useCallback((delta: number) => {
    setSettings((prev) => ({
      ...prev,
      fontSize: Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, prev.fontSize + delta)),
    }));
  }, []);

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay data-component="settings-overlay" />
        <Dialog.Content data-component="settings-dialog" aria-describedby={undefined}>
          <Dialog.Title data-slot="settings-title">
            <Settings className="h-4 w-4 text-muted-foreground" />
            Settings
          </Dialog.Title>

          <Dialog.Close data-slot="settings-close" aria-label="Close settings">
            <X className="h-4 w-4" />
          </Dialog.Close>

          <Tabs.Root defaultValue="general" data-slot="settings-tabs">
            <Tabs.List data-slot="settings-tab-list">
              <Tabs.Trigger value="general" data-slot="settings-tab">
                General
              </Tabs.Trigger>
              <Tabs.Trigger value="model" data-slot="settings-tab">
                Model
              </Tabs.Trigger>
              <Tabs.Trigger value="keybinds" data-slot="settings-tab">
                Keybinds
              </Tabs.Trigger>
              <Tabs.Trigger value="about" data-slot="settings-tab">
                About
              </Tabs.Trigger>
            </Tabs.List>

            {/* General tab */}
            <Tabs.Content value="general" data-slot="settings-panel">
              <div data-slot="settings-group">
                <span data-slot="settings-label">Mode</span>
                <div className="flex gap-2">
                  <button
                    type="button"
                    data-slot="settings-option"
                    data-active={theme === "light" || undefined}
                    onClick={() => {
                      if (theme === "dark") toggleTheme();
                    }}
                  >
                    Light
                  </button>
                  <button
                    type="button"
                    data-slot="settings-option"
                    data-active={theme === "dark" || undefined}
                    onClick={() => {
                      if (theme === "light") toggleTheme();
                    }}
                  >
                    Dark
                  </button>
                </div>
              </div>

              <div data-slot="settings-group">
                <span data-slot="settings-label">Color theme</span>
                <div data-slot="settings-theme-grid">
                  {THEMES.map((t) => (
                    <ThemeCard
                      key={t.name}
                      name={t.name}
                      label={t.label}
                      accent={t.accent}
                      bg={t.bg}
                      isActive={themeName === t.name}
                      onSelect={() => setThemeName(t.name)}
                    />
                  ))}
                </div>
              </div>

              <div data-slot="settings-group">
                <span data-slot="settings-label">
                  Font size
                  <span className="text-muted-foreground ml-1">{settings.fontSize}px</span>
                </span>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    data-slot="settings-stepper"
                    onClick={() => adjustFontSize(-1)}
                    disabled={settings.fontSize <= MIN_FONT_SIZE}
                    aria-label="Decrease font size"
                  >
                    <Minus className="h-3 w-3" />
                  </button>
                  <div
                    data-slot="settings-font-track"
                    className="flex-1 h-1.5 rounded-full bg-muted relative"
                  >
                    <div
                      className="absolute inset-y-0 left-0 rounded-full bg-accent"
                      style={{
                        width: `${((settings.fontSize - MIN_FONT_SIZE) / (MAX_FONT_SIZE - MIN_FONT_SIZE)) * 100}%`,
                      }}
                    />
                  </div>
                  <button
                    type="button"
                    data-slot="settings-stepper"
                    onClick={() => adjustFontSize(1)}
                    disabled={settings.fontSize >= MAX_FONT_SIZE}
                    aria-label="Increase font size"
                  >
                    <Plus className="h-3 w-3" />
                  </button>
                </div>
              </div>
            </Tabs.Content>

            {/* Model tab */}
            <Tabs.Content value="model" data-slot="settings-panel">
              <div data-slot="settings-group">
                <span data-slot="settings-label">Default model</span>
                {models && models.length > 0 ? (
                  <div className="flex flex-col gap-1">
                    {models.map((m) => (
                      <button
                        key={m.id}
                        type="button"
                        data-slot="settings-option"
                        data-active={selectedModel === m.id || undefined}
                        onClick={() => onModelChange?.(m.id)}
                      >
                        {m.name}
                      </button>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">
                    No models available. Configure authentication first.
                  </p>
                )}
              </div>
            </Tabs.Content>

            {/* Keybinds tab */}
            <Tabs.Content value="keybinds" data-slot="settings-panel">
              <div data-slot="settings-group">
                <span data-slot="settings-label">Session Management</span>
                <div data-slot="settings-keybinds-grid">
                  <ShortcutRow label="New session" keys={["Cmd", "N"]} />
                  <ShortcutRow label="Delete session" keys={["Cmd", "Shift", "Del"]} />
                  <ShortcutRow label="Settings" keys={["Cmd", ","]} />
                  <ShortcutRow label="Command palette" keys={["Cmd", "K"]} />
                </div>
              </div>

              <div data-slot="settings-group">
                <span data-slot="settings-label">Navigation</span>
                <div data-slot="settings-keybinds-grid">
                  <ShortcutRow label="Previous message" keys={["j"]} />
                  <ShortcutRow label="Next message" keys={["k"]} />
                  <ShortcutRow label="Focus editor" keys={["i"]} />
                  <ShortcutRow label="Abort / Blur editor" keys={["Esc"]} />
                </div>
              </div>

              <div data-slot="settings-group">
                <span data-slot="settings-label">Editing</span>
                <div data-slot="settings-keybinds-grid">
                  <ShortcutRow label="Send message" keys={["Enter"]} />
                  <ShortcutRow label="New line" keys={["Shift", "Enter"]} />
                </div>
              </div>
            </Tabs.Content>

            {/* About tab */}
            <Tabs.Content value="about" data-slot="settings-panel">
              <div data-slot="settings-group">
                <div className="flex flex-col gap-3 text-sm">
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Version</span>
                    <span className="font-mono text-xs">{VERSION}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Built with</span>
                    <span className="text-xs">React 19 + Tailwind v4 + Radix</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Engine</span>
                    <span className="text-xs">Claude Code (Anthropic)</span>
                  </div>
                </div>
              </div>
            </Tabs.Content>
          </Tabs.Root>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

// ── Theme card helper ────────────────────────────────────────────────────────

function ThemeCard({
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

// ── Shortcut row helper ──────────────────────────────────────────────────────

function ShortcutRow({ label, keys }: { label: string; keys: string[] }) {
  return (
    <>
      <span className="text-muted-foreground">{label}</span>
      <span className="flex items-center gap-0.5 justify-end">
        {keys.map((k) => (
          <kbd
            key={k}
            className={cn(
              "inline-flex items-center justify-center rounded px-1.5 py-0.5",
              "bg-muted border border-border text-[10px] font-mono text-muted-foreground",
              "min-w-[20px]",
            )}
          >
            {k}
          </kbd>
        ))}
      </span>
    </>
  );
}
