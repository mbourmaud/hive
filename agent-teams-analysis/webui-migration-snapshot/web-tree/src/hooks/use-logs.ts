import { useState, useEffect, useRef } from "react";
import { MOCK_LOGS } from "@/data/mock";

const MAX_LINES = 500;

export function useLogs(droneName: string | null, isMock: boolean) {
  const [logs, setLogs] = useState<string[]>([]);
  const eventSourceRef = useRef<EventSource | null>(null);
  const mockIntervalRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined);

  useEffect(() => {
    // Cleanup previous connections
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }
    if (mockIntervalRef.current) {
      clearInterval(mockIntervalRef.current);
      mockIntervalRef.current = undefined;
    }
    setLogs([]);

    if (!droneName) return;

    if (isMock) {
      const mockLines = MOCK_LOGS[droneName] || ["No logs available"];
      let idx = 0;

      // Seed initial lines
      const initial = mockLines.slice(0, 3);
      setLogs(initial);
      idx = initial.length;

      // Trickle remaining lines
      if (idx < mockLines.length) {
        mockIntervalRef.current = setInterval(() => {
          if (idx >= mockLines.length) idx = 0; // loop
          const line = mockLines[idx++];
          if (line) {
            setLogs((prev) => {
              const next = [...prev, line];
              return next.length > MAX_LINES ? next.slice(-MAX_LINES) : next;
            });
          }
        }, 1500);
      }
      return;
    }

    // Real SSE connection
    const es = new EventSource(`/api/logs/${encodeURIComponent(droneName)}`);
    eventSourceRef.current = es;

    es.onmessage = (event) => {
      setLogs((prev) => {
        const next = [...prev, event.data as string];
        return next.length > MAX_LINES ? next.slice(-MAX_LINES) : next;
      });
    };

    return () => {
      es.close();
      if (mockIntervalRef.current) {
        clearInterval(mockIntervalRef.current);
      }
    };
  }, [droneName, isMock]);

  return { logs };
}
