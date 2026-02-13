import { X } from "lucide-react";
import { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";
import { cn } from "@/shared/lib/utils";
import "./toast.css";

// ── Types ────────────────────────────────────────────────────────────────────

type ToastVariant = "success" | "error" | "info";

interface ToastItem {
  id: string;
  message: string;
  variant: ToastVariant;
  duration: number;
}

interface ToastContextValue {
  toast: (message: string, variant?: ToastVariant, duration?: number) => void;
}

// ── Context ──────────────────────────────────────────────────────────────────

const ToastContext = createContext<ToastContextValue | null>(null);

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within ToastProvider");
  return ctx;
}

// ── Provider ─────────────────────────────────────────────────────────────────

const MAX_TOASTS = 5;
const DEFAULT_DURATION = 3000;

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const counterRef = useRef(0);

  const toast = useCallback(
    (message: string, variant: ToastVariant = "info", duration = DEFAULT_DURATION) => {
      const id = `toast-${++counterRef.current}`;
      setToasts((prev) => {
        const next = [...prev, { id, message, variant, duration }];
        return next.length > MAX_TOASTS ? next.slice(-MAX_TOASTS) : next;
      });
    },
    [],
  );

  const dismiss = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  return (
    <ToastContext value={{ toast }}>
      {children}
      <div data-component="toast-container">
        {toasts.map((item) => (
          <ToastEntry key={item.id} item={item} onDismiss={dismiss} />
        ))}
      </div>
    </ToastContext>
  );
}

// ── Single toast entry ───────────────────────────────────────────────────────

function ToastEntry({ item, onDismiss }: { item: ToastItem; onDismiss: (id: string) => void }) {
  const [exiting, setExiting] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    timerRef.current = setTimeout(() => {
      setExiting(true);
    }, item.duration);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [item.duration]);

  useEffect(() => {
    if (!exiting) return;
    const id = setTimeout(() => onDismiss(item.id), 300);
    return () => clearTimeout(id);
  }, [exiting, item.id, onDismiss]);

  return (
    <output data-component="toast" data-variant={item.variant} data-exiting={exiting || undefined}>
      <span
        className={cn(
          "inline-block h-1.5 w-1.5 rounded-full shrink-0",
          item.variant === "success" && "bg-success",
          item.variant === "error" && "bg-destructive",
          item.variant === "info" && "bg-accent",
        )}
      />
      <span data-slot="toast-message">{item.message}</span>
      <button
        type="button"
        data-slot="toast-close"
        onClick={() => setExiting(true)}
        aria-label="Dismiss"
      >
        <X className="h-3 w-3" />
      </button>
    </output>
  );
}
