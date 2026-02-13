import "./session-turn.css"

import {
  useState,
  useEffect,
  useRef,
  useCallback,
  useMemo,
} from "react"
import { Copy, Check, ChevronDown, ChevronRight, Loader2 } from "lucide-react"
import { cn } from "@/lib/utils"
import { MarkdownRenderer } from "./markdown-renderer"
import type {
  ChatTurn,
  AssistantPart,
  ToolUsePart,
  ToolResultPart,
  TextPart,
} from "@/types/chat"

// ── Constants ────────────────────────────────────────────────────────────────

const COLLAPSE_CHAR_THRESHOLD = 200
const STATUS_DEBOUNCE_MS = 2500

// ── Status mapping ───────────────────────────────────────────────────────────

function computeStatusLabel(toolName: string): string {
  switch (toolName.toLowerCase()) {
    case "read":
    case "readfile":
      return "gathering context"
    case "grep":
    case "glob":
    case "search":
      return "searching codebase"
    case "edit":
    case "write":
    case "writefile":
      return "making edits"
    case "bash":
    case "execute":
    case "run":
      return "running commands"
    case "task":
    case "sendmessage":
    case "delegate":
      return "delegating"
    default:
      return "thinking"
  }
}

// ── Duration formatting ──────────────────────────────────────────────────────

function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  if (totalSeconds < 60) return `${totalSeconds}s`
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${minutes}m ${seconds}s`
}

// ── Hook: live elapsed time ──────────────────────────────────────────────────

function useElapsed(startedAt: number, isActive: boolean): number {
  const [elapsed, setElapsed] = useState(() =>
    isActive ? Date.now() - startedAt : 0,
  )

  useEffect(() => {
    if (!isActive) return

    const tick = () => setElapsed(Date.now() - startedAt)
    tick()
    const id = setInterval(tick, 1000)
    return () => clearInterval(id)
  }, [startedAt, isActive])

  return elapsed
}

// ── Hook: debounced status label ─────────────────────────────────────────────

function useDebouncedStatus(parts: AssistantPart[]): string {
  const [label, setLabel] = useState("thinking")
  const lastChangeRef = useRef(0)
  const pendingRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    // Find the last running tool_use part
    let lastToolName = ""
    for (let i = parts.length - 1; i >= 0; i--) {
      const part = parts[i]!
      if (part.type === "tool_use" && part.status === "running") {
        lastToolName = part.name
        break
      }
    }

    const newLabel = lastToolName ? computeStatusLabel(lastToolName) : "thinking"
    const now = Date.now()
    const timeSinceLastChange = now - lastChangeRef.current

    if (pendingRef.current) {
      clearTimeout(pendingRef.current)
      pendingRef.current = null
    }

    if (timeSinceLastChange >= STATUS_DEBOUNCE_MS) {
      setLabel(newLabel)
      lastChangeRef.current = now
    } else {
      const delay = STATUS_DEBOUNCE_MS - timeSinceLastChange
      pendingRef.current = setTimeout(() => {
        setLabel(newLabel)
        lastChangeRef.current = Date.now()
        pendingRef.current = null
      }, delay)
    }

    return () => {
      if (pendingRef.current) {
        clearTimeout(pendingRef.current)
        pendingRef.current = null
      }
    }
  }, [parts])

  return label
}

// ── Hook: sticky height tracking ─────────────────────────────────────────────

function useStickyHeight(): [React.RefObject<HTMLDivElement | null>, number] {
  const ref = useRef<HTMLDivElement>(null)
  const [height, setHeight] = useState(0)

  useEffect(() => {
    const el = ref.current
    if (!el) return

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setHeight(entry.contentRect.height)
      }
    })

    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  return [ref, height]
}

// ── PartRenderer (stub) ──────────────────────────────────────────────────────

interface PartRendererProps {
  part: AssistantPart
  result: ToolResultPart | undefined
}

function ToolStatusIcon({ status }: { status: ToolUsePart["status"] }) {
  switch (status) {
    case "running":
      return <Loader2 className="h-3.5 w-3.5 animate-spin text-accent" />
    case "completed":
      return <div className="h-2 w-2 rounded-full bg-success" />
    case "error":
      return <div className="h-2 w-2 rounded-full bg-destructive" />
    case "pending":
      return <div className="h-2 w-2 rounded-full bg-muted-foreground opacity-40" />
  }
}

function ToolInputSummary({ name, input }: { name: string; input: Record<string, unknown> }) {
  const lower = name.toLowerCase()

  // File operations — show shortened path
  if (["read", "readfile", "edit", "write", "writefile"].includes(lower)) {
    const filePath = input.file_path ?? input.path ?? ""
    if (typeof filePath === "string" && filePath) {
      const short = filePath.split("/").slice(-2).join("/")
      return <span className="text-muted-foreground truncate">...{short}</span>
    }
  }

  // Search operations — show pattern
  if (["grep", "glob", "search"].includes(lower)) {
    const pattern = input.pattern ?? input.query ?? ""
    if (typeof pattern === "string" && pattern) {
      return (
        <span className="font-mono text-muted-foreground truncate text-xs">
          {pattern.length > 40 ? pattern.slice(0, 40) + "..." : pattern}
        </span>
      )
    }
  }

  // Bash — show command
  if (lower === "bash") {
    const cmd = input.command ?? ""
    if (typeof cmd === "string" && cmd) {
      return (
        <span className="font-mono text-muted-foreground truncate text-xs">
          {cmd.length > 50 ? cmd.slice(0, 50) + "..." : cmd}
        </span>
      )
    }
  }

  return null
}

function PartRenderer({ part, result }: PartRendererProps) {
  const [expanded, setExpanded] = useState(false)

  if (part.type === "text") {
    return (
      <div data-slot="session-turn-part-text">
        <MarkdownRenderer text={part.text} />
      </div>
    )
  }

  if (part.type === "tool_use") {
    return (
      <div data-slot="session-turn-part-tool">
        <button
          type="button"
          data-slot="session-turn-tool-header"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
        >
          <div className="flex items-center gap-2 min-w-0">
            <ToolStatusIcon status={part.status} />
            <span className="font-medium text-foreground shrink-0">{part.name}</span>
            <ToolInputSummary name={part.name} input={part.input} />
          </div>
          <ChevronRight
            className={cn(
              "h-3.5 w-3.5 text-muted-foreground shrink-0 transition-transform duration-150",
              expanded && "rotate-90",
            )}
          />
        </button>

        {expanded && (
          <div data-slot="session-turn-tool-body">
            <pre className="text-xs font-mono overflow-x-auto">
              <code>{JSON.stringify(part.input, null, 2)}</code>
            </pre>
            {result && (
              <div data-slot="session-turn-tool-result" data-error={result.isError || undefined}>
                <pre className="text-xs font-mono overflow-x-auto whitespace-pre-wrap">
                  <code>
                    {result.content.length > 2000
                      ? result.content.slice(0, 2000) + "\n... (truncated)"
                      : result.content}
                  </code>
                </pre>
              </div>
            )}
          </div>
        )}
      </div>
    )
  }

  // tool_result parts are rendered inline with their tool_use — skip standalone
  return null
}

// ── SessionTurn ──────────────────────────────────────────────────────────────

interface SessionTurnProps {
  turn: ChatTurn
  isLast: boolean
  stepsExpanded: boolean
  onToggleSteps: () => void
}

export function SessionTurn({
  turn,
  isLast,
  stepsExpanded,
  onToggleSteps,
}: SessionTurnProps) {
  const [userExpanded, setUserExpanded] = useState(false)
  const [copied, setCopied] = useState(false)
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const isStreaming = turn.status === "streaming"
  const elapsed = useElapsed(turn.startedAt, isStreaming)
  const statusLabel = useDebouncedStatus(turn.assistantParts)
  const [stickyRef, stickyHeight] = useStickyHeight()

  // ── Derived data ─────────────────────────────────────────────────────────

  const canExpandUser = turn.userMessage.length > COLLAPSE_CHAR_THRESHOLD

  const toolUseParts = useMemo(
    () => turn.assistantParts.filter((p): p is ToolUsePart => p.type === "tool_use"),
    [turn.assistantParts],
  )

  const toolResultMap = useMemo(() => {
    const map = new Map<string, ToolResultPart>()
    for (const p of turn.assistantParts) {
      if (p.type === "tool_result") {
        map.set(p.toolUseId, p)
      }
    }
    return map
  }, [turn.assistantParts])

  const stepsCount = toolUseParts.length

  // Summary = last text part (regardless of whether tools exist before it)
  const summaryText = useMemo(() => {
    const lastTextIdx = findLastTextIndex(turn.assistantParts)
    if (lastTextIdx === -1) return null
    return (turn.assistantParts[lastTextIdx] as TextPart).text
  }, [turn.assistantParts])

  let displayDuration: string | null = null
  if (turn.duration !== null) {
    displayDuration = formatDuration(turn.duration)
  } else if (isStreaming) {
    displayDuration = formatDuration(elapsed)
  }

  // ── Error text ───────────────────────────────────────────────────────────

  const errorText = useMemo(() => {
    const errorResults = turn.assistantParts.filter(
      (p): p is ToolResultPart => p.type === "tool_result" && p.isError,
    )
    if (errorResults.length > 0) {
      return errorResults.map((r) => r.content).join("\n\n")
    }
    return turn.status === "error" ? "An error occurred during this turn." : null
  }, [turn.assistantParts, turn.status])

  // ── Steps content (tool_use parts in order) ──────────────────────────────

  const stepsParts = useMemo(() => {
    const lastTextIdx = findLastTextIndex(turn.assistantParts)
    const result: AssistantPart[] = []
    for (let i = 0; i < turn.assistantParts.length; i++) {
      const part = turn.assistantParts[i]!
      if (part.type === "tool_use") {
        result.push(part)
      } else if (part.type === "text" && i !== lastTextIdx) {
        result.push(part)
      }
    }
    return result
  }, [turn.assistantParts])

  // ── Copy handler ─────────────────────────────────────────────────────────

  const handleCopy = useCallback(() => {
    if (!summaryText) return
    navigator.clipboard.writeText(summaryText).then(() => {
      setCopied(true)
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current)
      copyTimeoutRef.current = setTimeout(() => setCopied(false), 2000)
    })
  }, [summaryText])

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current)
    }
  }, [])

  // ── Render ───────────────────────────────────────────────────────────────

  return (
    <div
      data-component="session-turn"
      data-status={turn.status}
      data-last={isLast || undefined}
      style={
        { "--session-turn-sticky-height": `${stickyHeight}px` } as React.CSSProperties
      }
    >
      <div data-slot="session-turn-content">
        <div data-slot="session-turn-message-container">
          {/* ── Sticky header: user message + steps toggle ─────────────── */}
          <div data-slot="session-turn-sticky" ref={stickyRef}>
            <div
              data-slot="session-turn-message-content"
              data-can-expand={canExpandUser || undefined}
              data-expanded={userExpanded || undefined}
            >
              <p>{turn.userMessage}</p>

              {canExpandUser && (
                <button
                  type="button"
                  data-slot="session-turn-expand-btn"
                  onClick={() => setUserExpanded((prev) => !prev)}
                >
                  {userExpanded ? "Show less" : "Show more"}
                </button>
              )}
            </div>

            {stepsCount > 0 && (
              <button
                type="button"
                data-slot="session-turn-response-trigger"
                onClick={onToggleSteps}
                aria-expanded={stepsExpanded}
              >
                <div className="flex items-center gap-2">
                  {isStreaming ? (
                    <Loader2 className="h-3.5 w-3.5 animate-spin text-accent" />
                  ) : (
                    <ChevronDown
                      className={cn(
                        "h-3.5 w-3.5 text-muted-foreground transition-transform duration-150",
                        !stepsExpanded && "-rotate-90",
                      )}
                    />
                  )}
                  <span className="text-foreground font-medium">
                    {stepsCount} {stepsCount === 1 ? "step" : "steps"}
                  </span>
                  {isStreaming && (
                    <span className="text-muted-foreground">{statusLabel}</span>
                  )}
                </div>
                {displayDuration && (
                  <span className="text-muted-foreground text-xs tabular-nums">
                    {displayDuration}
                  </span>
                )}
              </button>
            )}
          </div>

          {/* ── Steps (collapsible tool call list) ────────────────────── */}
          {stepsCount > 0 && stepsExpanded && (
            <div data-slot="session-turn-steps">
              {stepsParts.map((part) => (
                <PartRenderer
                  key={part.id}
                  part={part}
                  result={
                    part.type === "tool_use"
                      ? toolResultMap.get(part.id)
                      : undefined
                  }
                />
              ))}
            </div>
          )}

          {/* ── Summary (final assistant text) ────────────────────────── */}
          {summaryText && !isStreaming && (
            <div
              data-slot="session-turn-summary"
              data-fade={isLast || undefined}
            >
              <MarkdownRenderer text={summaryText} />
              <button
                type="button"
                data-slot="session-turn-copy"
                onClick={handleCopy}
                aria-label="Copy response"
              >
                {copied ? (
                  <Check className="h-3.5 w-3.5 text-success" />
                ) : (
                  <Copy className="h-3.5 w-3.5" />
                )}
              </button>
            </div>
          )}

          {/* ── Streaming summary (live) ──────────────────────────────── */}
          {summaryText && isStreaming && (
            <div data-slot="session-turn-summary" data-streaming="true">
              <MarkdownRenderer text={summaryText} />
            </div>
          )}

          {/* ── Error ─────────────────────────────────────────────────── */}
          {errorText && (
            <div data-slot="session-turn-error">
              <pre>{errorText}</pre>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function findLastTextIndex(parts: AssistantPart[]): number {
  for (let i = parts.length - 1; i >= 0; i--) {
    if (parts[i]!.type === "text") return i
  }
  return -1
}
