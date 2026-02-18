import { useCallback, useEffect, useRef } from "react";
import { apiClient } from "@/shared/api/client";
import { safeFetch } from "@/shared/api/safe-fetch";
import { useAppStore } from "@/store";
import { coalesceEvents, isStreamEvent } from "./event-coalescing";
import type { ChatSession, ImageAttachment, StreamEvent } from "./types";

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
  const generationRef = useRef(0);

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

    // Auto-open Plans tab when HivePlan session completes
    const hasSessionCompleted = coalesced.some((e) => e.type === "session.completed");
    if (hasSessionCompleted) {
      const { chatMode, openRightSidebar } = useAppStore.getState();
      if (chatMode === "hive-plan") {
        openRightSidebar("plans");
      }
    }
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
    (sessionId: string, turnId: string): Promise<void> => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
        eventSourceRef.current = null;
      }

      const url = `${baseUrl}/api/chat/sessions/${sessionId}/stream`;
      return new Promise<void>((resolve) => {
        const es = new EventSource(url);
        eventSourceRef.current = es;
        lastEventTimeRef.current = Date.now();

        const gen = ++generationRef.current;
        es.onmessage = (msg) => {
          if (generationRef.current !== gen) return;
          try {
            const parsed: unknown = JSON.parse(msg.data);
            if (isStreamEvent(parsed)) {
              enqueueEvent(parsed);
            }
          } catch {
            // skip malformed lines
          }
        };

        es.onopen = () => resolve();

        es.onerror = () => {
          resolve(); // Don't block sendMessage forever
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
      });
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
      dispatchChat({ type: "TURN_STARTED", turnId, userMessage: message, model, images });

      abortControllerRef.current?.abort();
      const controller = new AbortController();
      abortControllerRef.current = controller;

      await connectToSession(session.id, turnId);

      const effort = useAppStore.getState().effort;
      const chatMode = useAppStore.getState().chatMode;
      const result = await safeFetch(`${baseUrl}/api/chat/sessions/${session.id}/message`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          text: message,
          model,
          effort,
          mode: chatMode,
          images:
            images?.map((img) => ({
              data: img.dataUrl.replace(/^data:[^;]+;base64,/, ""),
              media_type: img.mimeType,
            })) ?? [],
        }),
        signal: controller.signal,
      });

      if (result.ok) return;

      switch (result.type) {
        case "aborted":
          return;
        case "network":
        case "api":
          dispatchChat({ type: "TURN_ERROR", turnId, error: result.message });
          return;
        default: {
          const _exhaustive: never = result;
          return _exhaustive;
        }
      }
    },
    [baseUrl, dispatchChat, connectToSession],
  );

  // ── Abort current request ───────────────────────────────────────────────

  const abort = useCallback(() => {
    // Bump generation so in-flight SSE callbacks are ignored
    generationRef.current++;

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

    // Flush pending RAF queue to prevent stale events dispatching after abort
    const queue = queueRef.current;
    if (queue.rafId !== null) {
      cancelAnimationFrame(queue.rafId);
      queue.rafId = null;
    }
    queue.events = [];

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

  // ── Disconnect SSE (no abort POST, no TURN_ERROR) ────────────────────

  const disconnect = useCallback(() => {
    eventSourceRef.current?.close();
    eventSourceRef.current = null;
    if (retryTimeoutRef.current) {
      clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = undefined;
    }
    stopHeartbeatCheck();
    generationRef.current++;
    const queue = queueRef.current;
    if (queue.rafId !== null) {
      cancelAnimationFrame(queue.rafId);
      queue.rafId = null;
    }
    queue.events = [];
  }, [stopHeartbeatCheck]);

  // ── Reset ─────────────────────────────────────────────────────────────

  const resetSession = useCallback(() => {
    disconnect();
    dispatchChat({ type: "SESSION_RESET" });
  }, [disconnect, dispatchChat]);

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
    disconnect,
    connectToSession,
  };
}
