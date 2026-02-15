import { Skeleton } from "@/shared/ui/skeleton";

const GROUP_A = ["sk-a-1", "sk-a-2", "sk-a-3", "sk-a-4"] as const;
const GROUP_B = ["sk-b-1", "sk-b-2"] as const;

/** Skeleton placeholder for session list items during project switch. */
export function SessionListSkeleton() {
  return (
    <div className="flex flex-col px-1.5">
      {/* Group header skeleton */}
      <div className="px-2.5 pt-3 pb-1">
        <Skeleton width={48} height={10} />
      </div>

      {/* Session item skeletons */}
      <div className="flex flex-col gap-0.5">
        {GROUP_A.map((id, i) => (
          <div
            key={id}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-md"
            style={{ animationDelay: `${i * 100}ms` }}
          >
            <Skeleton variant="circle" width={6} height={6} />
            <Skeleton width={`${75 - i * 12}%`} />
          </div>
        ))}
      </div>

      {/* Second group */}
      <div className="px-2.5 pt-3 pb-1">
        <Skeleton width={64} height={10} />
      </div>
      <div className="flex flex-col gap-0.5">
        {GROUP_B.map((id, i) => (
          <div
            key={id}
            className="flex items-center gap-2 px-2.5 py-1.5 rounded-md"
            style={{ animationDelay: `${(i + 4) * 100}ms` }}
          >
            <Skeleton variant="circle" width={6} height={6} />
            <Skeleton width={`${60 - i * 10}%`} />
          </div>
        ))}
      </div>
    </div>
  );
}
