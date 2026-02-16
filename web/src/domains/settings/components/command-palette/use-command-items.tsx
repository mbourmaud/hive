import { HelpCircle, Layers, MessageSquare, Plus, Settings, Sun, Trash2, Zap } from "lucide-react";
import { useMemo } from "react";

// ── Types ────────────────────────────────────────────────────────────────────

export interface CommandItem {
  id: string;
  group: "actions" | "commands" | "sessions";
  icon: React.ReactNode;
  label: string;
  description: string;
  shortcut?: string[];
  onSelect: () => void;
}

interface UseCommandItemsOptions {
  sessions: { id: string; title: string }[];
  onSelectSession: (id: string) => void;
  onNewSession: () => void;
  onClearChat: () => void;
  onOpenSettings: () => void;
  onToggleTheme: () => void;
  onRunCommand: (command: string) => void;
}

// ── Hook ─────────────────────────────────────────────────────────────────────

export function useCommandItems({
  sessions,
  onSelectSession,
  onNewSession,
  onClearChat,
  onOpenSettings,
  onToggleTheme,
  onRunCommand,
}: UseCommandItemsOptions): CommandItem[] {
  return useMemo((): CommandItem[] => {
    const actions: CommandItem[] = [
      {
        id: "action-new-session",
        group: "actions",
        icon: <Plus className="w-4 h-4" />,
        label: "New Session",
        description: "Start a new chat session",
        shortcut: ["\u2318", "N"],
        onSelect: onNewSession,
      },
      {
        id: "action-settings",
        group: "actions",
        icon: <Settings className="w-4 h-4" />,
        label: "Settings",
        description: "Open settings dialog",
        shortcut: ["\u2318", ","],
        onSelect: onOpenSettings,
      },
      {
        id: "action-toggle-theme",
        group: "actions",
        icon: <Sun className="w-4 h-4" />,
        label: "Toggle Theme",
        description: "Switch between light and dark mode",
        onSelect: onToggleTheme,
      },
      {
        id: "action-clear-chat",
        group: "actions",
        icon: <Trash2 className="w-4 h-4" />,
        label: "Clear Chat",
        description: "Clear the current conversation",
        onSelect: onClearChat,
      },
    ];

    const commands: CommandItem[] = [
      {
        id: "cmd-clear",
        group: "commands",
        icon: <Trash2 className="w-4 h-4" />,
        label: "/clear",
        description: "Clear conversation history",
        onSelect: () => onRunCommand("/clear"),
      },
      {
        id: "cmd-compact",
        group: "commands",
        icon: <Layers className="w-4 h-4" />,
        label: "/compact",
        description: "Summarize and continue",
        onSelect: () => onRunCommand("/compact"),
      },
      {
        id: "cmd-model",
        group: "commands",
        icon: <Zap className="w-4 h-4" />,
        label: "/model",
        description: "Switch model (e.g. /model sonnet)",
        onSelect: () => onRunCommand("/model"),
      },
      {
        id: "cmd-help",
        group: "commands",
        icon: <HelpCircle className="w-4 h-4" />,
        label: "/help",
        description: "Show available commands",
        onSelect: () => onRunCommand("/help"),
      },
    ];

    const sessionItems: CommandItem[] = sessions.map((s) => ({
      id: `session-${s.id}`,
      group: "sessions" as const,
      icon: <MessageSquare className="w-4 h-4" />,
      label: s.title,
      description: "Switch to session",
      onSelect: () => onSelectSession(s.id),
    }));

    return [...actions, ...commands, ...sessionItems];
  }, [
    sessions,
    onNewSession,
    onOpenSettings,
    onToggleTheme,
    onClearChat,
    onRunCommand,
    onSelectSession,
  ]);
}
