import * as Dialog from "@radix-ui/react-dialog";
import * as Tabs from "@radix-ui/react-tabs";
import { Settings, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  exportCurrentTheme,
  generateThemeId,
  validateThemeFile,
} from "@/shared/theme/custom-theme";
import { useTheme } from "@/shared/theme/use-theme";
import { useAppStore } from "@/store";
import { GeneralTab } from "./general-tab";
import { KeybindsTab } from "./keybinds-tab";
import { ProfilesTab } from "./profiles-tab";
import {
  type AppSettings,
  loadSettings,
  MAX_FONT_SIZE,
  MIN_FONT_SIZE,
  saveSettings,
} from "./storage";
import "./settings-dialog.css";
import "./theme-grid.css";
import "./widgets.css";

// ── Constants ────────────────────────────────────────────────────────────────

const VERSION = "0.1.0";

// ── Types ────────────────────────────────────────────────────────────────────

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  models?: { id: string; name: string }[];
  selectedModel?: string;
  onModelChange?: (modelId: string) => void;
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
  const [importError, setImportError] = useState<string | null>(null);

  const customThemes = useAppStore((s) => s.customThemes);
  const activeCustomThemeId = useAppStore((s) => s.activeCustomThemeId);
  const addCustomTheme = useAppStore((s) => s.addCustomTheme);
  const removeCustomTheme = useAppStore((s) => s.removeCustomTheme);
  const setActiveCustomTheme = useAppStore((s) => s.setActiveCustomTheme);

  const fileInputRef = useRef<HTMLInputElement>(null);

  // Apply font size via CSS custom properties on :root
  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--font-size-base", `${settings.fontSize}px`);
    root.style.setProperty("--font-size-code", `${settings.fontSize - 1}px`);
    saveSettings(settings);
  }, [settings]);

  const adjustFontSize = useCallback((delta: number) => {
    setSettings((prev) => ({
      ...prev,
      fontSize: Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, prev.fontSize + delta)),
    }));
  }, []);

  const handleImportClick = useCallback(() => {
    setImportError(null);
    fileInputRef.current?.click();
  }, []);

  const handleFileChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = () => {
        try {
          const parsed: unknown = JSON.parse(reader.result as string);
          const result = validateThemeFile(parsed);
          if (!result.ok) {
            setImportError(result.error);
            return;
          }

          const id = generateThemeId(result.theme.name);
          addCustomTheme({ ...result.theme, id });
          setImportError(null);
        } catch {
          setImportError("Invalid JSON file");
        }
      };
      reader.readAsText(file);
      e.target.value = "";
    },
    [addCustomTheme],
  );

  const handleExport = useCallback(() => {
    const themeData = exportCurrentTheme("My Custom Theme");
    const blob = new Blob([JSON.stringify(themeData, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "hive-theme.json";
    a.click();
    URL.revokeObjectURL(url);
  }, []);

  const handleDeleteCustomTheme = useCallback(
    (id: string) => {
      removeCustomTheme(id);
    },
    [removeCustomTheme],
  );

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
              <Tabs.Trigger value="profiles" data-slot="settings-tab">
                Profiles
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

            <Tabs.Content value="general" data-slot="settings-panel">
              <GeneralTab
                theme={theme}
                toggleTheme={toggleTheme}
                themeName={themeName}
                setThemeName={setThemeName}
                customThemes={customThemes}
                activeCustomThemeId={activeCustomThemeId}
                setActiveCustomTheme={setActiveCustomTheme}
                handleDeleteCustomTheme={handleDeleteCustomTheme}
                handleImportClick={handleImportClick}
                handleExport={handleExport}
                importError={importError}
                fileInputRef={fileInputRef}
                handleFileChange={handleFileChange}
                settings={settings}
                adjustFontSize={adjustFontSize}
              />
            </Tabs.Content>

            <Tabs.Content value="profiles" data-slot="settings-panel">
              <ProfilesTab />
            </Tabs.Content>

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

            <Tabs.Content value="keybinds" data-slot="settings-panel">
              <KeybindsTab />
            </Tabs.Content>

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
