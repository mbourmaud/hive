import { X } from "lucide-react";
import type { QueuedMessage as QueuedMessageType } from "../types";
import "./queued-message.css";

interface QueuedMessageProps {
  message: QueuedMessageType;
  onCancel: (id: string) => void;
}

export function QueuedMessage({ message, onCancel }: QueuedMessageProps) {
  return (
    <div data-component="queued-message">
      <div data-slot="queued-message-header">
        <span data-slot="queued-message-badge">QUEUED</span>
        <button
          type="button"
          data-slot="queued-message-cancel"
          onClick={() => onCancel(message.id)}
          aria-label="Cancel queued message"
        >
          <X className="h-3 w-3" />
        </button>
      </div>
      <p data-slot="queued-message-text">{message.text}</p>
      {message.images && message.images.length > 0 && (
        <div data-slot="queued-message-images">
          {message.images.map((img) => (
            <img key={img.id} src={img.dataUrl} alt={img.name} data-slot="queued-message-thumb" />
          ))}
        </div>
      )}
    </div>
  );
}
