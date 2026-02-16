import { useEffect, useRef } from "react";
import { useAppStore } from "@/store";
import type { ImageAttachment } from "../types";

interface UseMessageDequeueOptions {
  handleSend: (message: string, images?: ImageAttachment[]) => void;
}

/**
 * Watches `isStreaming` falling edge (true→false) and auto-sends
 * the next queued message when the current turn completes.
 */
export function useMessageDequeue({ handleSend }: UseMessageDequeueOptions) {
  const isStreaming = useAppStore((s) => s.isStreaming);
  const prevStreamingRef = useRef(isStreaming);

  useEffect(() => {
    const wasStreaming = prevStreamingRef.current;
    prevStreamingRef.current = isStreaming;

    if (!wasStreaming || isStreaming) return;

    // Streaming just stopped — check the queue
    const { messageQueue, dispatchChat } = useAppStore.getState();
    const next = messageQueue[0];
    if (!next) return;

    // Dequeue first, then send
    dispatchChat({ type: "DEQUEUE_MESSAGE" });

    // Use microtask so the dequeue state settles before the send triggers a new turn
    queueMicrotask(() => {
      handleSend(next.text, next.images);
    });
  }, [isStreaming, handleSend]);
}
