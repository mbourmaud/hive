import { useState, useEffect, useRef, useCallback } from "react";
import type { DroneInfo } from "@/types/api";
import { MOCK_DRONES } from "@/data/mock";

type ConnectionStatus = "connected" | "disconnected" | "mock";

export function useDrones() {
  const [drones, setDrones] = useState<DroneInfo[]>([]);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>("disconnected");
  const eventSourceRef = useRef<EventSource | null>(null);
  const retryTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const connectSSE = useCallback(() => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }

    const es = new EventSource("/api/events");
    eventSourceRef.current = es;

    es.onopen = () => {
      setConnectionStatus("connected");
    };

    es.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as DroneInfo[];
        setDrones(data);
        setConnectionStatus("connected");
      } catch {
        // ignore parse errors
      }
    };

    es.onerror = () => {
      es.close();
      setConnectionStatus("disconnected");
      // Retry after 3 seconds
      retryTimeoutRef.current = setTimeout(connectSSE, 3000);
    };
  }, []);

  useEffect(() => {
    // Initial fetch
    fetch("/api/drones")
      .then((res) => {
        if (!res.ok) throw new Error("No backend");
        return res.json();
      })
      .then((data: DroneInfo[]) => {
        setDrones(data);
        connectSSE();
      })
      .catch(() => {
        // Dev mode — use mock data
        console.info("No backend detected, using mock data for preview");
        setDrones(MOCK_DRONES);
        setConnectionStatus("mock");
      });

    return () => {
      eventSourceRef.current?.close();
      if (retryTimeoutRef.current) {
        clearTimeout(retryTimeoutRef.current);
      }
    };
  }, [connectSSE]);

  return { drones, connectionStatus };
}
