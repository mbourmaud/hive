import { useEffect, useRef, useState } from "react";
import { MOCK_LOGS } from "@/shared/data/mock";

const MAX_LINES = 500;

export function useLogs(
  droneName: string | null,
  isMock: boolean,
  projectPath?: string,
  raw = false,
) {
  const [logs, setLogs] = useState<string[]>([]);
  const eventSourceRef = useRef<EventSource | null>(null);
  const mockIntervalRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined);

  useEffect(() => {
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

    function appendLog(line: string) {
      setLogs((prev) => {
        const next = [...prev, line];
        return next.length > MAX_LINES ? next.slice(-MAX_LINES) : next;
      });
    }

    if (isMock) {
      const mockLines = MOCK_LOGS[droneName] ?? ["No logs available"];
      let idx = 0;

      const initial = mockLines.slice(0, 3);
      setLogs(initial);
      idx = initial.length;

      if (idx < mockLines.length) {
        mockIntervalRef.current = setInterval(() => {
          if (idx >= mockLines.length) idx = 0;
          const line = mockLines[idx++];
          if (line) appendLog(line);
        }, 1500);
      }
    } else {
      const base = projectPath
        ? `/api/logs/${encodeURIComponent(projectPath)}/${encodeURIComponent(droneName)}`
        : `/api/logs/${encodeURIComponent(droneName)}`;
      const logsUrl = raw ? `${base}?format=raw` : base;
      const es = new EventSource(logsUrl);
      eventSourceRef.current = es;

      es.onmessage = (event: MessageEvent<string>) => {
        appendLog(event.data);
      };
    }

    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
        eventSourceRef.current = null;
      }
      if (mockIntervalRef.current) {
        clearInterval(mockIntervalRef.current);
        mockIntervalRef.current = undefined;
      }
    };
  }, [droneName, isMock, projectPath, raw]);

  return { logs };
}
