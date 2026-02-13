import { useCallback, useEffect, useRef } from "react";
import { useChatReducer } from "@/hooks/use-chat-reducer";
import type { ChatSession, StreamEvent } from "@/types/chat";

// ── Constants ───────────────────────────────────────────────────────────────

const SSE_RETRY_MS = 250;
const HEARTBEAT_TIMEOUT_MS = 35_000; // mark stale if no event in 35s

// ── Event queue with RAF coalescing ─────────────────────────────────────────

interface EventQueue {
  events: StreamEvent[];
  rafId: number | null;
}

// ── Hook ────────────────────────────────────────────────────────────────────

export function useChat(baseUrl: string = "") {
  const [state, dispatch] = useChatReducer();

  const eventSourceRef = useRef<EventSource | null>(null);
  const retryTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  const heartbeatRef = useRef<ReturnType<typeof setInterval> | undefined>(
    undefined,
  );
  const queueRef = useRef<EventQueue>({ events: [], rafId: null });
  const lastEventTimeRef = useRef<number>(0);
  const abortControllerRef = useRef<AbortController | null>(null);
  const stateRef = useRef(state);
  stateRef.current = state;

  // ── Flush queued events via requestAnimationFrame ───────────────────────

  const flushQueue = useCallback(() => {
    const queue = queueRef.current;
    if (queue.events.length === 0) {
      queue.rafId = null;
      return;
    }

    // Coalesce: for events updating the same part, keep only the latest
    const coalesced = coalesceEvents(queue.events);
    queue.events = [];
    queue.rafId = null;

    dispatch({ type: "STREAM_EVENT_BATCH", events: coalesced });
  }, [dispatch]);

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
        dispatch({ type: "MARK_STALE" });
      }
    }, 5_000);
  }, [dispatch]);

  const stopHeartbeatCheck = useCallback(() => {
    if (heartbeatRef.current) {
      clearInterval(heartbeatRef.current);
      heartbeatRef.current = undefined;
    }
  }, []);

  // ── SSE connection ──────────────────────────────────────────────────────

  const connectToSession = useCallback(
    (sessionId: string, turnId: string) => {
      // Close existing connection
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
          const event = JSON.parse(msg.data) as StreamEvent;
          enqueueEvent(event);
        } catch {
          // skip malformed lines
        }
      };

      es.onerror = () => {
        es.close();
        eventSourceRef.current = null;

        // Auto-reconnect after delay (read latest state via ref to avoid stale closure)
        retryTimeoutRef.current = setTimeout(() => {
          const current = stateRef.current;
          if (current.isStreaming && current.currentTurnId === turnId) {
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
    async (cwd: string): Promise<ChatSession> => {
      const res = await fetch(`${baseUrl}/api/chat/sessions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ cwd }),
      });

      if (!res.ok) {
        const text = await res.text();
        throw new Error(`Failed to create session: ${text}`);
      }

      const session = (await res.json()) as ChatSession;
      dispatch({ type: "SESSION_CREATED", session });
      return session;
    },
    [baseUrl, dispatch],
  );

  // ── Send a message ──────────────────────────────────────────────────────

  const sendMessage = useCallback(
    async (message: string) => {
      if (!state.session) {
        throw new Error("No active session. Call createSession() first.");
      }

      const turnId = `turn-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      dispatch({ type: "TURN_STARTED", turnId, userMessage: message });

      // Cancel any in-flight request
      abortControllerRef.current?.abort();
      const controller = new AbortController();
      abortControllerRef.current = controller;

      try {
        const res = await fetch(
          `${baseUrl}/api/chat/sessions/${state.session.id}/message`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ text: message }),
            signal: controller.signal,
          },
        );

        if (!res.ok) {
          const text = await res.text();
          dispatch({ type: "TURN_ERROR", turnId, error: text });
          return;
        }

        // Connect SSE for streaming response
        connectToSession(state.session.id, turnId);
      } catch (err) {
        if (err instanceof DOMException && err.name === "AbortError") {
          return; // user-initiated abort
        }
        const errorMsg =
          err instanceof Error ? err.message : "Unknown error";
        dispatch({ type: "TURN_ERROR", turnId, error: errorMsg });
      }
    },
    [baseUrl, state.session, dispatch, connectToSession],
  );

  // ── Abort current request ───────────────────────────────────────────────

  const abort = useCallback(() => {
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

    if (state.currentTurnId) {
      dispatch({
        type: "TURN_ERROR",
        turnId: state.currentTurnId,
        error: "Aborted by user",
      });
    }
  }, [dispatch, state.currentTurnId, stopHeartbeatCheck]);

  // ── Cleanup on unmount ──────────────────────────────────────────────────

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
    state,
    sendMessage,
    abort,
    createSession,
  };
}

// ── Event coalescing ────────────────────────────────────────────────────────

/** Coalesce consecutive text-only assistant events into a single event. */
function coalesceEvents(events: StreamEvent[]): StreamEvent[] {
  if (events.length <= 1) return events;

  const result: StreamEvent[] = [];
  let pendingText = "";

  for (let i = 0; i < events.length; i++) {
    const event = events[i]!;

    if (
      event.type === "assistant" &&
      event.message.content.length === 1 &&
      event.message.content[0]!.type === "text"
    ) {
      // Accumulate consecutive text-only assistant events
      pendingText += event.message.content[0]!.text;
    } else {
      // Flush accumulated text first
      if (pendingText) {
        result.push({
          type: "assistant",
          message: { content: [{ type: "text", text: pendingText }] },
        });
        pendingText = "";
      }
      result.push(event);
    }
  }

  // Flush any remaining text
  if (pendingText) {
    result.push({
      type: "assistant",
      message: { content: [{ type: "text", text: pendingText }] },
    });
  }

  return result;
}
