import { ShortcutRow } from "./shortcut-row";

export function KeybindsTab() {
  return (
    <>
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
    </>
  );
}
