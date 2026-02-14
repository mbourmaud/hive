import { cn } from "@/shared/lib/utils";

export function ShortcutRow({ label, keys }: { label: string; keys: string[] }) {
  return (
    <>
      <span className="text-muted-foreground">{label}</span>
      <span className="flex items-center gap-0.5 justify-end">
        {keys.map((k) => (
          <kbd
            key={k}
            className={cn(
              "inline-flex items-center justify-center rounded px-1.5 py-0.5",
              "bg-muted border border-border text-[10px] font-mono text-muted-foreground",
              "min-w-[20px]",
            )}
          >
            {k}
          </kbd>
        ))}
      </span>
    </>
  );
}
