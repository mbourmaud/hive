import { Download, Minus, Plus, Upload } from "lucide-react";
import type { ThemeName } from "@/shared/theme/use-theme";
import { THEMES } from "@/shared/theme/use-theme";
import type { AppSettings } from "./storage";
import { MAX_FONT_SIZE, MIN_FONT_SIZE } from "./storage";
import { CustomThemeCard, ThemeCard } from "./theme-card";

interface GeneralTabProps {
  theme: string;
  toggleTheme: () => void;
  themeName: string;
  setThemeName: (name: ThemeName) => void;
  customThemes: { id: string; name: string; dark: { accent: string; background: string } }[];
  activeCustomThemeId: string | null;
  setActiveCustomTheme: (id: string) => void;
  handleDeleteCustomTheme: (id: string) => void;
  handleImportClick: () => void;
  handleExport: () => void;
  importError: string | null;
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  handleFileChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  settings: AppSettings;
  adjustFontSize: (delta: number) => void;
}

export function GeneralTab({
  theme,
  toggleTheme,
  themeName,
  setThemeName,
  customThemes,
  activeCustomThemeId,
  setActiveCustomTheme,
  handleDeleteCustomTheme,
  handleImportClick,
  handleExport,
  importError,
  fileInputRef,
  handleFileChange,
  settings,
  adjustFontSize,
}: GeneralTabProps) {
  return (
    <>
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
              isActive={!activeCustomThemeId && themeName === t.name}
              onSelect={() => setThemeName(t.name)}
            />
          ))}
          {customThemes.map((ct) => (
            <CustomThemeCard
              key={ct.id}
              id={ct.id}
              label={ct.name}
              accent={ct.dark.accent}
              bg={ct.dark.background}
              isActive={activeCustomThemeId === ct.id}
              onSelect={() => setActiveCustomTheme(ct.id)}
              onDelete={() => handleDeleteCustomTheme(ct.id)}
            />
          ))}
        </div>
      </div>

      <div data-slot="settings-group">
        <span data-slot="settings-label">Custom themes</span>
        <div className="flex gap-2">
          <button type="button" data-slot="settings-theme-action" onClick={handleImportClick}>
            <Upload className="h-3.5 w-3.5" />
            Import
          </button>
          <button type="button" data-slot="settings-theme-action" onClick={handleExport}>
            <Download className="h-3.5 w-3.5" />
            Export current
          </button>
        </div>
        {importError && <p data-slot="settings-error">{importError}</p>}
        <input
          ref={fileInputRef}
          type="file"
          accept=".json"
          className="hidden"
          onChange={handleFileChange}
        />
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
    </>
  );
}
