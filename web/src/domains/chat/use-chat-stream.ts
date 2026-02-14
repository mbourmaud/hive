import { useCallback, useEffect, useRef } from "react";
import { apiClient } from "@/shared/api/client";
import { useAppStore } from "@/store";
import type { ChatSession, ImageAttachment, StreamEvent } from "./types";
import { coalesceEvents, isStreamEvent } from "./event-coalescing";

// ── Constants ───────────────────────────────────────────────────────────────

const SSE_RETRY_MS = 250;
const HEARTBEAT_TIMEOUT_MS = 35_000;

// ── Event queue with RAF coalescing ─────────────────────────────────────────

interface EventQueue {
  events: StreamEvent[];
  rafId: number | null;
}

// ── Hook ────────────────────────────────────────────────────────────────────

export function useChat(baseUrl: string = "") {
  const dispatchChat = useAppStore((s) => s.dispatchChat);

  const eventSourceRef = useRef<EventSource | null>(null);
  const retryTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const heartbeatRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined);
  const queueRef = useRef<EventQueue>({ events: [], rafId: null });
  const lastEventTimeRef = useRef<number>(0);
  const abortControllerRef = useRef<AbortController | null>(null);

  // ── Flush queued events via requestAnimationFrame ───────────────────────

  const flushQueue = useCallback(() => {
    const queue = queueRef.current;
    if (queue.events.length === 0) {
      queue.rafId = null;
      return;
    }

    const coalesced = coalesceEvents(queue.events);
    queue.events = [];
    queue.rafId = null;

    dispatchChat({ type: "STREAM_EVENT_BATCH", events: coalesced });
  }, [dispatchChat]);

  const enqueueEvent = useCallback(
    (event: StreamEvent) => {
      const queue = queueRef.current;
      queue.events.push(event);
      lastEventTimeRef.current = Date.now();

      if (queue.rafId === null) {
        queue.rafId = requestAnimationFrame(flushQueue);
      }
    },
    [flushQueue],
  );

  // ── Heartbeat detection ─────────────────────────────────────────────────

  const startHeartbeatCheck = useCallback(() => {
    if (heartbeatRef.current) clearInterval(heartbeatRef.current);
    heartbeatRef.current = setInterval(() => {
      const elapsed = Date.now() - lastEventTimeRef.current;
      if (lastEventTimeRef.current > 0 && elapsed > HEARTBEAT_TIMEOUT_MS) {
        dispatchChat({ type: "MARK_STALE" });
      }
    }, 5_000);
  }, [dispatchChat]);

  const stopHeartbeatCheck = useCallback(() => {
    if (heartbeatRef.current) {
      clearInterval(heartbeatRef.current);
      heartbeatRef.current = undefined;
    }
  }, []);

  // ── SSE connection ──────────────────────────────────────────────────────

  const connectToSession = useCallback(
    (sessionId: string, turnId: string) => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
        eventSourceRef.current = null;
      }

      const url = `${baseUrl}/api/chat/sessions/${sessionId}/stream`;
      const es = new EventSource(url);
      eventSourceRef.current = es;
      lastEventTimeRef.current = Date.now();

      es.onmessage = (msg) => {
        try {
          const parsed: unknown = JSON.parse(msg.data);
          if (isStreamEvent(parsed)) {
            enqueueEvent(parsed);
          }
        } catch {
          // skip malformed lines
        }
      };

      es.onerror = () => {
        es.close();
        eventSourceRef.current = null;

        retryTimeoutRef.current = setTimeout(() => {
          const store = useAppStore.getState();
          if (store.isStreaming && store.currentTurnId === turnId) {
            connectToSession(sessionId, turnId);
          }
        }, SSE_RETRY_MS);
      };

      startHeartbeatCheck();
    },
    [baseUrl, enqueueEvent, startHeartbeatCheck],
  );

  // ── Create a new session ────────────────────────────────────────────────

  const createSession = useCallback(
    async (cwd: string, model?: string): Promise<ChatSession> => {
      const session = await apiClient.post<ChatSession>(`${baseUrl}/api/chat/sessions`, {
        cwd,
        model: model ?? "sonnet",
      });
      dispatchChat({ type: "SESSION_CREATED", session });
      return session;
    },
    [baseUrl, dispatchChat],
  );

  // ── Send a message ──────────────────────────────────────────────────────

  const sendMessage = useCallback(
    async (
      message: string,
      sessionOverride?: ChatSession,
      model?: string,
      images?: ImageAttachment[],
    ) => {
      const session = sessionOverride ?? useAppStore.getState().session;
      if (!session) {
        throw new Error("No active session. Call createSession() first.");
      }

      const turnId = `turn-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      dispatchChat({ type: "TURN_STARTED", turnId, userMessage: message, model });

      abortControllerRef.current?.abort();
      const controller = new AbortController();
      abortControllerRef.current = controller;

      connectToSession(session.id, turnId);

      try {
        const effort = useAppStore.getState().effort;
        const res = await fetch(`${baseUrl}/api/chat/sessions/${session.id}/message`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            text: message,
            model,
            effort,
            images:
              images?.map((img) => ({
                data: img.dataUrl.replace(/^data:[^;]+;base64,/, ""),
                media_type: img.mimeType,
              })) ?? [],
          }),
          signal: controller.signal,
        });

        if (!res.ok) {
          const text = await res.text();
          dispatchChat({ type: "TURN_ERROR", turnId, error: text });
        }
      } catch (err) {
        if (err instanceof DOMException && err.name === "AbortError") {
          return;
        }
        const errorMsg = err instanceof Error ? err.message : "Unknown error";
        dispatchChat({ type: "TURN_ERROR", turnId, error: errorMsg });
      }
    },
    [baseUrl, dispatchChat, connectToSession],
  );

  // ── Abort current request ───────────────────────────────────────────────

  const abort = useCallback(() => {
    const session = useAppStore.getState().session;
    if (session) {
      fetch(`${baseUrl}/api/chat/sessions/${session.id}/abort`, {
        method: "POST",
      }).catch(() => {
        /* best effort */
      });
    }

    abortControllerRef.current?.abort();
    abortControllerRef.current = null;

    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }

    if (retryTimeoutRef.current) {
      clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = undefined;
    }

    stopHeartbeatCheck();

    const currentTurnId = useAppStore.getState().currentTurnId;
    if (currentTurnId) {
      dispatchChat({
        type: "TURN_ERROR",
        turnId: currentTurnId,
        error: "Aborted by user",
      });
    }
  }, [baseUrl, dispatchChat, stopHeartbeatCheck]);

  // ── Reset ─────────────────────────────────────────────────────────────

  const resetSession = useCallback(() => {
    dispatchChat({ type: "SESSION_RESET" });
  }, [dispatchChat]);

  // ── Cleanup on unmount ────────────────────────────────────────────────

  useEffect(() => {
    return () => {
      eventSourceRef.current?.close();
      if (retryTimeoutRef.current) clearTimeout(retryTimeoutRef.current);
      stopHeartbeatCheck();
      const queue = queueRef.current;
      if (queue.rafId !== null) cancelAnimationFrame(queue.rafId);
    };
  }, [stopHeartbeatCheck]);

  return {
    sendMessage,
    abort,
    createSession,
    resetSession,
  };
}
