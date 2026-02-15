import { Skeleton } from "@/shared/ui/skeleton";

/** Skeleton placeholder for ContextBar while project detection runs. */
export function ContextBarSkeleton() {
  return (
    <div data-component="context-bar">
      {/* Git branch skeleton */}
      <div className="flex items-center gap-1.5">
        <Skeleton variant="circle" width={14} height={14} />
        <Skeleton width={100} />
      </div>

      {/* Runtime pill skeletons */}
      <Skeleton variant="pill" width={72} />
      <Skeleton variant="pill" width={60} />
    </div>
  );
}
