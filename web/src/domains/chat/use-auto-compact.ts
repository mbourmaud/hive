import { useEffect, useRef } from "react";
import { useToast } from "@/shared/ui/toast";
import { useAppStore } from "@/store";

const AUTO_COMPACT_THRESHOLD = 160_000; // 80% of 200K context window

/**
 * Triggers automatic conversation compaction when context usage exceeds
 * 80% of the model's input limit (160K of 200K tokens). This prevents
 * the truncation system from silently dropping messages and instead
 * summarizes the conversation via the /compact API.
 */
export function useAutoCompact() {
  const { toast } = useToast();
  const inputTokens = useAppStore((s) => s.contextUsage?.inputTokens ?? 0);
  const sessionId = useAppStore((s) => s.session?.id ?? null);
  const sessionStatus = useAppStore((s) => s.session?.status ?? null);
  const isStreaming = useAppStore((s) => s.isStreaming);
  const dispatchChat = useAppStore((s) => s.dispatchChat);
  const setActiveSession = useAppStore((s) => s.setActiveSession);

  const triggeredForRef = useRef<string | null>(null);
  const pendingRef = useRef(false);

  useEffect(() => {
    // Reset trigger flag when session changes so auto-compact can fire again
    if (triggeredForRef.current !== null && triggeredForRef.current !== sessionId) {
      triggeredForRef.current = null;
    }

    if (!sessionId || isStreaming || pendingRef.current) return;
    if (inputTokens < AUTO_COMPACT_THRESHOLD) return;
    if (triggeredForRef.current === sessionId) return;
    if (sessionStatus !== "idle") return;

    triggeredForRef.current = sessionId;
    pendingRef.current = true;

    toast("Auto-compacting conversation (80% context used)", "info");

    fetch(`/api/chat/sessions/${sessionId}/compact?auto=true`, { method: "POST" })
      .then((res) => {
        if (res.ok) {
          toast("Conversation auto-compacted", "success");
          dispatchChat({ type: "SESSION_RESET" });
          setActiveSession(null);
          queueMicrotask(() => setActiveSession(sessionId));
        } else {
          toast("Auto-compact failed", "error");
        }
      })
      .catch(() => {
        toast("Auto-compact failed", "error");
      })
      .finally(() => {
        pendingRef.current = false;
      });
  }, [sessionId, inputTokens, isStreaming, sessionStatus, toast, dispatchChat, setActiveSession]);
}
