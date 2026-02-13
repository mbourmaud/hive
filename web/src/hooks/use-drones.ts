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
    // Force mock mode with ?mock query param
    const forceMock = new URLSearchParams(window.location.search).has("mock");
    if (forceMock) {
      console.info("Mock mode forced via ?mock query param");
      setDrones(MOCK_DRONES);
      setConnectionStatus("mock");
      return;
    }

    // Initial fetch with timeout so dev mode falls back quickly
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);

    fetch("/api/drones", { signal: controller.signal })
      .then((res) => {
        clearTimeout(timeout);
        if (!res.ok) throw new Error("No backend");
        return res.json();
      })
      .then((data: DroneInfo[]) => {
        setDrones(data);
        connectSSE();
      })
      .catch(() => {
        clearTimeout(timeout);
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
