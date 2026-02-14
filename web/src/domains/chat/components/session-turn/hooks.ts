import { useEffect, useRef, useState } from "react";
import type { AssistantPart } from "../../types";

// ── Constants ────────────────────────────────────────────────────────────────

const STATUS_DEBOUNCE_MS = 2500;

// ── Status mapping ───────────────────────────────────────────────────────────

function computeStatusLabel(toolName: string): string {
  switch (toolName.toLowerCase()) {
    case "read":
    case "readfile":
      return "gathering context";
    case "grep":
    case "glob":
    case "search":
      return "searching codebase";
    case "edit":
    case "write":
    case "writefile":
      return "making edits";
    case "bash":
    case "execute":
    case "run":
      return "running commands";
    case "task":
    case "sendmessage":
    case "delegate":
      return "delegating";
    default:
      return "thinking";
  }
}

// ── Hook: live elapsed time ──────────────────────────────────────────────────

export function useElapsed(startedAt: number, isActive: boolean): number {
  const [elapsed, setElapsed] = useState(() => (isActive ? Date.now() - startedAt : 0));

  useEffect(() => {
    if (!isActive) return;

    const tick = () => setElapsed(Date.now() - startedAt);
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [startedAt, isActive]);

  return elapsed;
}

// ── Hook: debounced status label ─────────────────────────────────────────────

export function useDebouncedStatus(parts: AssistantPart[]): string {
  const [label, setLabel] = useState("thinking");
  const lastChangeRef = useRef(0);
  const pendingRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let lastToolName = "";
    for (let i = parts.length - 1; i >= 0; i--) {
      const part = parts[i];
      if (part?.type === "tool_use" && part.status === "running") {
        lastToolName = part.name;
        break;
      }
    }

    const newLabel = lastToolName ? computeStatusLabel(lastToolName) : "thinking";
    const now = Date.now();
    const timeSinceLastChange = now - lastChangeRef.current;

    if (pendingRef.current) {
      clearTimeout(pendingRef.current);
      pendingRef.current = null;
    }

    if (timeSinceLastChange >= STATUS_DEBOUNCE_MS) {
      setLabel(newLabel);
      lastChangeRef.current = now;
    } else {
      const delay = STATUS_DEBOUNCE_MS - timeSinceLastChange;
      pendingRef.current = setTimeout(() => {
        setLabel(newLabel);
        lastChangeRef.current = Date.now();
        pendingRef.current = null;
      }, delay);
    }

    return () => {
      if (pendingRef.current) {
        clearTimeout(pendingRef.current);
        pendingRef.current = null;
      }
    };
  }, [parts]);

  return label;
}

// ── Hook: sticky height tracking ─────────────────────────────────────────────

export function useStickyHeight(): [React.RefObject<HTMLDivElement | null>, number] {
  const ref = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState(0);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setHeight(entry.contentRect.height);
      }
    });

    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return [ref, height];
}
