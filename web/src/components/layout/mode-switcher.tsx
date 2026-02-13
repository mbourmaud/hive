import { useCallback, useEffect } from "react"
import { MessageSquareText, MonitorDot } from "lucide-react"
import { cn } from "@/lib/utils"

export type AppMode = "chat" | "monitor"

interface ModeSwitcherProps {
  mode: AppMode
  onModeChange: (mode: AppMode) => void
}

const MODES: { key: AppMode; label: string; icon: typeof MessageSquareText }[] = [
  { key: "chat", label: "Chat", icon: MessageSquareText },
  { key: "monitor", label: "Monitor", icon: MonitorDot },
]

export function ModeSwitcher({ mode, onModeChange }: ModeSwitcherProps) {
  // Keyboard shortcuts: Cmd/Ctrl+1 → Chat, Cmd/Ctrl+2 → Monitor
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return
      if (e.key === "1") {
        e.preventDefault()
        onModeChange("chat")
      } else if (e.key === "2") {
        e.preventDefault()
        onModeChange("monitor")
      }
    },
    [onModeChange],
  )

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [handleKeyDown])

  return (
    <div
      data-component="mode-switcher"
      className="inline-flex items-center rounded-lg bg-muted p-0.5 gap-0.5"
      role="tablist"
    >
      {MODES.map(({ key, label, icon: Icon }) => (
        <button
          key={key}
          type="button"
          role="tab"
          aria-selected={mode === key}
          onClick={() => onModeChange(key)}
          className={cn(
            "inline-flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium transition-all duration-150",
            mode === key
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          <Icon className="h-3.5 w-3.5" />
          {label}
        </button>
      ))}
    </div>
  )
}
