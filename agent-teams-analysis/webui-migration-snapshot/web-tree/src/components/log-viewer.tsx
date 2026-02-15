import { useEffect, useRef } from "react";

interface LogViewerProps {
  logs: string[];
}

export function LogViewer({ logs }: LogViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <div
      ref={containerRef}
      className="max-h-[350px] overflow-y-auto rounded-lg p-3 font-mono text-xs leading-relaxed"
      style={{
        background: "var(--bg)",
        border: "1px solid var(--border)",
        color: "var(--text-muted)",
      }}
    >
      {logs.length === 0 ? (
        <div>Waiting for logs...</div>
      ) : (
        logs.map((line, i) => <div key={i}>{line}</div>)
      )}
    </div>
  );
}
