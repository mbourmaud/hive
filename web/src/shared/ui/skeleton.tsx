import "./skeleton.css";

interface SkeletonProps {
  width?: string | number;
  height?: string | number;
  variant?: "text" | "circle" | "pill";
  className?: string;
  style?: React.CSSProperties;
}

/** Animated placeholder for loading states. */
export function Skeleton({ width, height, variant = "text", className, style }: SkeletonProps) {
  const resolved: React.CSSProperties = {
    width: typeof width === "number" ? `${width}px` : width,
    height: typeof height === "number" ? `${height}px` : height,
    ...style,
  };

  return (
    <div data-component="skeleton" data-variant={variant} className={className} style={resolved} />
  );
}
