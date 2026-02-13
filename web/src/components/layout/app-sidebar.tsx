import {
  useState,
  useCallback,
  useRef,
  useEffect,
} from "react"
import { Plus, GripVertical } from "lucide-react"
import { cn } from "@/lib/utils"
import { ThemeToggle } from "@/components/theme-toggle"
import { DroneItem } from "@/components/drone-item"
import type { AppMode } from "./mode-switcher"
import { ModeSwitcher } from "./mode-switcher"
import type { DroneInfo } from "@/types/api"
import beeIcon from "@/assets/bee-icon.png"

// ── Constants ────────────────────────────────────────────────────────────────

const SIDEBAR_MIN = 200
const SIDEBAR_MAX = 400
const SIDEBAR_DEFAULT = 280
const COLLAPSE_THRESHOLD = 160

// ── Types ────────────────────────────────────────────────────────────────────

interface AppSidebarProps {
  mode: AppMode
  onModeChange: (mode: AppMode) => void
  // Chat mode
  sessions: SessionEntry[]
  activeSessionId: string | null
  onSelectSession: (id: string) => void
  onNewSession: () => void
  // Monitor mode
  drones: DroneInfo[]
  selectedDrone: string | null
  onSelectDrone: (name: string) => void
  connectionStatus: "connected" | "disconnected" | "mock"
  projectName?: string
}

export interface SessionEntry {
  id: string
  title: string
  createdAt: string
  status: "idle" | "busy" | "completed" | "error"
}

// ── Sidebar resize hook ──────────────────────────────────────────────────────

function useSidebarResize(defaultWidth: number) {
  const [width, setWidth] = useState(defaultWidth)
  const [collapsed, setCollapsed] = useState(false)
  const dragging = useRef(false)
  const startX = useRef(0)
  const startWidth = useRef(0)

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      dragging.current = true
      startX.current = e.clientX
      startWidth.current = collapsed ? SIDEBAR_MIN : width
      document.body.style.cursor = "col-resize"
      document.body.style.userSelect = "none"
    },
    [width, collapsed],
  )

  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      if (!dragging.current) return
      const delta = e.clientX - startX.current
      const newWidth = startWidth.current + delta

      if (newWidth < COLLAPSE_THRESHOLD) {
        setCollapsed(true)
      } else {
        setCollapsed(false)
        setWidth(Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, newWidth)))
      }
    }

    const onMouseUp = () => {
      if (!dragging.current) return
      dragging.current = false
      document.body.style.cursor = ""
      document.body.style.userSelect = ""
    }

    window.addEventListener("mousemove", onMouseMove)
    window.addEventListener("mouseup", onMouseUp)
    return () => {
      window.removeEventListener("mousemove", onMouseMove)
      window.removeEventListener("mouseup", onMouseUp)
    }
  }, [])

  return { width: collapsed ? 0 : width, collapsed, onMouseDown, setCollapsed }
}

// ── SSE indicator ────────────────────────────────────────────────────────────

function sseDisplayInfo(status: string): { dotClass: string; label: string } {
  switch (status) {
    case "connected":
      return { dotClass: "bg-success", label: "Live" }
    case "mock":
      return { dotClass: "bg-warning", label: "Mock" }
    default:
      return { dotClass: "bg-destructive", label: "Offline" }
  }
}

function SseIndicator({ status }: { status: string }) {
  const { dotClass, label } = sseDisplayInfo(status)
  return (
    <div className="flex items-center gap-1.5">
      <div className={cn("w-1.5 h-1.5 rounded-full", dotClass)} />
      <span className="text-[11px] text-muted-foreground">{label}</span>
    </div>
  )
}

// ── Component ────────────────────────────────────────────────────────────────

export function AppSidebar({
  mode,
  onModeChange,
  sessions,
  activeSessionId,
  onSelectSession,
  onNewSession,
  drones,
  selectedDrone,
  onSelectDrone,
  connectionStatus,
  projectName,
}: AppSidebarProps) {
  const { width, collapsed, onMouseDown } = useSidebarResize(SIDEBAR_DEFAULT)

  if (collapsed) {
    return (
      <nav
        data-component="app-sidebar"
        data-collapsed
        className="w-12 flex flex-col items-center py-3 gap-3 shrink-0 bg-sidebar border-r border-sidebar-border"
      >
        <img src={beeIcon} alt="Hive" className="w-6 h-6 opacity-60" />
      </nav>
    )
  }

  return (
    <>
      <nav
        data-component="app-sidebar"
        className="flex flex-col shrink-0 bg-sidebar border-r border-sidebar-border relative"
        style={{ width: `${width}px` }}
      >
        {/* Header */}
        <div className="h-12 flex items-center gap-2 px-3 shrink-0 border-b border-sidebar-border">
          <img src={beeIcon} alt="Hive" className="w-6 h-6 shrink-0" />
          <span className="text-sm font-extrabold tracking-wider text-accent font-mono">
            HIVE
          </span>
          <div className="ml-auto flex items-center gap-2">
            <ThemeToggle />
          </div>
        </div>

        {/* Mode switcher */}
        <div className="px-3 py-2 border-b border-sidebar-border">
          <ModeSwitcher mode={mode} onModeChange={onModeChange} />
        </div>

        {/* Content area — mode dependent */}
        <div className="flex-1 overflow-y-auto">
          {mode === "chat" ? (
            <ChatSidebarContent
              sessions={sessions}
              activeSessionId={activeSessionId}
              onSelectSession={onSelectSession}
              onNewSession={onNewSession}
            />
          ) : (
            <MonitorSidebarContent
              drones={drones}
              selectedDrone={selectedDrone}
              onSelectDrone={onSelectDrone}
              projectName={projectName}
            />
          )}
        </div>

        {/* Footer — connection status (monitor only) */}
        {mode === "monitor" && (
          <div className="px-3 py-2 border-t border-sidebar-border">
            <SseIndicator status={connectionStatus} />
          </div>
        )}
      </nav>

      {/* Drag handle */}
      <div
        data-slot="sidebar-drag-handle"
        className="w-1 cursor-col-resize shrink-0 relative group"
        onMouseDown={onMouseDown}
      >
        <div className="absolute inset-y-0 -left-0.5 -right-0.5 flex items-center justify-center">
          <GripVertical className="h-4 w-4 text-border opacity-0 group-hover:opacity-60 transition-opacity" />
        </div>
        <div className="absolute inset-y-0 left-0 w-px bg-sidebar-border group-hover:bg-accent/40 transition-colors" />
      </div>
    </>
  )
}

// ── Chat sidebar content ─────────────────────────────────────────────────────

function ChatSidebarContent({
  sessions,
  activeSessionId,
  onSelectSession,
  onNewSession,
}: {
  sessions: SessionEntry[]
  activeSessionId: string | null
  onSelectSession: (id: string) => void
  onNewSession: () => void
}) {
  return (
    <div className="flex flex-col">
      {/* New session button */}
      <button
        type="button"
        onClick={onNewSession}
        className="mx-3 mt-3 mb-2 inline-flex items-center gap-2 rounded-lg border border-dashed border-border px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:border-foreground/20 transition-colors"
      >
        <Plus className="h-3.5 w-3.5" />
        New session
      </button>

      {/* Session list */}
      {sessions.length === 0 ? (
        <div className="px-3 py-4 text-xs text-muted-foreground">
          No sessions yet. Start a new one.
        </div>
      ) : (
        <div className="flex flex-col gap-0.5 px-1.5">
          {sessions.map((session) => (
            <button
              key={session.id}
              type="button"
              onClick={() => onSelectSession(session.id)}
              className={cn(
                "w-full text-left rounded-md px-2.5 py-2 text-sm transition-colors",
                activeSessionId === session.id
                  ? "bg-accent/10 text-foreground"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground",
              )}
            >
              <div className="flex items-center gap-2 min-w-0">
                <div
                  className={cn(
                    "w-1.5 h-1.5 rounded-full shrink-0",
                    session.status === "busy" && "bg-accent",
                    session.status === "idle" && "bg-success",
                    session.status === "error" && "bg-destructive",
                    session.status === "completed" && "bg-muted-foreground",
                  )}
                />
                <span className="truncate">{session.title}</span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

// ── Monitor sidebar content ──────────────────────────────────────────────────

function MonitorSidebarContent({
  drones,
  selectedDrone,
  onSelectDrone,
  projectName,
}: {
  drones: DroneInfo[]
  selectedDrone: string | null
  onSelectDrone: (name: string) => void
  projectName?: string
}) {
  return (
    <div className="flex flex-col">
      {/* Project name header */}
      {projectName && (
        <div className="px-3 py-2 border-b border-sidebar-border">
          <div className="flex items-center gap-2 min-w-0">
            <div className="w-1.5 h-1.5 rounded-full bg-accent shrink-0" />
            <span className="text-sm font-semibold text-foreground truncate">
              {projectName}
            </span>
            <span className="text-xs text-muted-foreground shrink-0">
              ({drones.length})
            </span>
          </div>
        </div>
      )}

      {/* Drone list */}
      {drones.length === 0 ? (
        <div className="p-4 text-sm text-muted-foreground">
          No drones detected.
        </div>
      ) : (
        drones.map((drone) => (
          <DroneItem
            key={drone.name}
            drone={drone}
            selected={selectedDrone === drone.name}
            onClick={() => onSelectDrone(drone.name)}
          />
        ))
      )}
    </div>
  )
}
