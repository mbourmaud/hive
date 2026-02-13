import {
  useState,
  useEffect,
  useRef,
  useCallback,
  useMemo,
} from "react"
import { ArrowDown } from "lucide-react"
import { cn } from "@/lib/utils"
import { SessionTurn } from "./session-turn"
import { PromptInput } from "./prompt-input"
import type { ChatTurn } from "@/types/chat"
import beeIcon from "@/assets/bee-icon.png"

// ── Constants ────────────────────────────────────────────────────────────────

const INITIAL_RENDER_COUNT = 20
const SCROLL_THRESHOLD = 100 // px from bottom to consider "at bottom"

// ── Types ────────────────────────────────────────────────────────────────────

interface ChatLayoutProps {
  turns: ChatTurn[]
  isStreaming: boolean
  error: string | null
  currentTurnId: string | null
  onSend: (message: string) => void
  onAbort: () => void
  hasSession: boolean
}

// ── Auto-scroll hook ─────────────────────────────────────────────────────────

function useAutoScroll(turns: ChatTurn[], isStreaming: boolean) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const isUserScrolling = useRef(false)
  const wasAtBottom = useRef(true)

  const isAtBottom = useCallback(() => {
    const el = scrollRef.current
    if (!el) return true
    return el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD
  }, [])

  const scrollToBottom = useCallback((smooth = true) => {
    const el = scrollRef.current
    if (!el) return
    el.scrollTo({
      top: el.scrollHeight,
      behavior: smooth ? "smooth" : "instant",
    })
    isUserScrolling.current = false
    wasAtBottom.current = true
  }, [])

  // Detect user scroll gestures
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return

    const onWheel = () => {
      isUserScrolling.current = true
    }
    const onTouchStart = () => {
      isUserScrolling.current = true
    }
    const onScroll = () => {
      wasAtBottom.current = isAtBottom()
      if (wasAtBottom.current) {
        isUserScrolling.current = false
      }
    }

    el.addEventListener("wheel", onWheel, { passive: true })
    el.addEventListener("touchstart", onTouchStart, { passive: true })
    el.addEventListener("scroll", onScroll, { passive: true })

    return () => {
      el.removeEventListener("wheel", onWheel)
      el.removeEventListener("touchstart", onTouchStart)
      el.removeEventListener("scroll", onScroll)
    }
  }, [isAtBottom])

  // Auto-scroll on new content if user hasn't scrolled away
  useEffect(() => {
    if (!isUserScrolling.current && wasAtBottom.current) {
      scrollToBottom(false)
    }
  }, [turns, isStreaming, scrollToBottom])

  return { scrollRef, scrollToBottom }
}

// ── Progressive rendering hook ───────────────────────────────────────────────

function useProgressiveRender(turns: ChatTurn[]) {
  const [renderCount, setRenderCount] = useState(INITIAL_RENDER_COUNT)

  // Reset when turns array changes drastically (new session)
  useEffect(() => {
    if (turns.length <= INITIAL_RENDER_COUNT) {
      setRenderCount(INITIAL_RENDER_COUNT)
    }
  }, [turns.length])

  // Backfill older turns via requestIdleCallback
  useEffect(() => {
    if (renderCount >= turns.length) return

    const id = requestIdleCallback(
      () => {
        setRenderCount((prev) => Math.min(prev + 10, turns.length))
      },
      { timeout: 500 },
    )

    return () => cancelIdleCallback(id)
  }, [renderCount, turns.length])

  const visibleTurns = useMemo(() => {
    if (turns.length <= renderCount) return turns
    const startIdx = Math.max(0, turns.length - renderCount)
    return turns.slice(startIdx)
  }, [turns, renderCount])

  return { visibleTurns, isBackfilling: renderCount < turns.length }
}

// ── Scroll button state hook (separate to avoid re-renders) ──────────────────

function useScrollButtonVisibility(
  scrollRef: React.RefObject<HTMLDivElement | null>,
) {
  const [visible, setVisible] = useState(false)

  useEffect(() => {
    const el = scrollRef.current
    if (!el) return

    const onScroll = () => {
      const atBottom =
        el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD
      setVisible(!atBottom)
    }

    el.addEventListener("scroll", onScroll, { passive: true })
    return () => el.removeEventListener("scroll", onScroll)
  }, [scrollRef])

  return visible
}

// ── Component ────────────────────────────────────────────────────────────────

export function ChatLayout({
  turns,
  isStreaming,
  error,
  currentTurnId,
  onSend,
  onAbort,
  hasSession,
}: ChatLayoutProps) {
  // Steps expansion state — track per-turn
  const [expandedSteps, setExpandedSteps] = useState<Set<string>>(new Set())

  const toggleSteps = useCallback((turnId: string) => {
    setExpandedSteps((prev) => {
      const next = new Set(prev)
      if (next.has(turnId)) {
        next.delete(turnId)
      } else {
        next.add(turnId)
      }
      return next
    })
  }, [])

  // Auto-expand steps for the current streaming turn
  useEffect(() => {
    if (currentTurnId && isStreaming) {
      setExpandedSteps((prev) => {
        if (prev.has(currentTurnId)) return prev
        const next = new Set(prev)
        next.add(currentTurnId)
        return next
      })
    }
  }, [currentTurnId, isStreaming])

  const { visibleTurns } = useProgressiveRender(turns)
  const { scrollRef, scrollToBottom } = useAutoScroll(turns, isStreaming)
  const showScrollBtn = useScrollButtonVisibility(scrollRef)

  // Current turn status for prompt input
  const currentTurn = turns.find((t) => t.id === currentTurnId)
  const turnStatus = currentTurn?.status ?? null

  if (!hasSession && turns.length === 0) {
    return (
      <div data-component="chat-view" className="flex-1 flex flex-col relative overflow-hidden bg-background">
        {/* Empty state */}
        <div className="flex-1 flex flex-col items-center justify-center gap-4 px-4">
          <img src={beeIcon} alt="Hive" className="w-14 h-14 opacity-20" />
          <div className="text-center">
            <p className="text-lg font-medium text-muted-foreground">
              Start a conversation
            </p>
            <p className="text-sm text-muted-foreground/60 mt-1">
              Ask anything. Claude Code will help you build.
            </p>
          </div>
        </div>

        {/* Prompt dock */}
        <PromptInput
          onSend={onSend}
          onAbort={onAbort}
          isStreaming={isStreaming}
          error={error}
          turnStatus={turnStatus}
        />
      </div>
    )
  }

  return (
    <div data-component="chat-view" className="flex-1 flex flex-col relative overflow-hidden bg-background">
      {/* Message list */}
      <div
        ref={scrollRef}
        data-slot="message-list"
        className="flex-1 overflow-y-auto"
      >
        <div className="max-w-[900px] mx-auto px-4 sm:px-6 pb-[calc(var(--prompt-height,8rem)+64px)]">
          {visibleTurns.map((turn, idx) => (
            <SessionTurn
              key={turn.id}
              turn={turn}
              isLast={idx === visibleTurns.length - 1}
              stepsExpanded={expandedSteps.has(turn.id)}
              onToggleSteps={() => toggleSteps(turn.id)}
            />
          ))}
        </div>
      </div>

      {/* Scroll to bottom button */}
      {showScrollBtn && (
        <button
          type="button"
          data-slot="scroll-to-bottom"
          onClick={() => scrollToBottom(true)}
          className={cn(
            "absolute bottom-32 left-1/2 -translate-x-1/2 z-40",
            "inline-flex items-center gap-1.5 rounded-full px-3 py-1.5",
            "bg-card border border-border shadow-lg",
            "text-xs text-muted-foreground hover:text-foreground",
            "transition-all duration-200 hover:shadow-xl",
          )}
        >
          <ArrowDown className="h-3.5 w-3.5" />
          New messages
        </button>
      )}

      {/* Prompt dock */}
      <PromptInput
        onSend={onSend}
        onAbort={onAbort}
        isStreaming={isStreaming}
        disabled={false}
        error={error}
        turnStatus={turnStatus}
      />
    </div>
  )
}
