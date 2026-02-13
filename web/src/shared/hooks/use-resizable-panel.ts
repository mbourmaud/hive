import { useCallback, useEffect, useRef, useState } from "react";

interface UseResizablePanelOptions {
  minWidth: number;
  maxWidth: number;
  defaultWidth: number;
  collapseThreshold: number;
  side: "left" | "right";
}

interface UseResizablePanelReturn {
  width: number;
  collapsed: boolean;
  onMouseDown: (e: React.MouseEvent) => void;
  setCollapsed: (collapsed: boolean) => void;
}

export function useResizablePanel({
  minWidth,
  maxWidth,
  defaultWidth,
  collapseThreshold,
  side,
}: UseResizablePanelOptions): UseResizablePanelReturn {
  const [width, setWidth] = useState(defaultWidth);
  const [collapsed, setCollapsed] = useState(false);
  const dragging = useRef(false);
  const startX = useRef(0);
  const startWidth = useRef(0);

  const onMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragging.current = true;
      startX.current = e.clientX;
      startWidth.current = collapsed ? minWidth : width;
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [width, collapsed, minWidth],
  );

  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      if (!dragging.current) return;
      const rawDelta = e.clientX - startX.current;
      // For right-side panels, dragging left (negative delta) increases width
      const delta = side === "right" ? -rawDelta : rawDelta;
      const newWidth = startWidth.current + delta;

      if (newWidth < collapseThreshold) {
        setCollapsed(true);
      } else {
        setCollapsed(false);
        setWidth(Math.min(maxWidth, Math.max(minWidth, newWidth)));
      }
    };

    const onMouseUp = () => {
      if (!dragging.current) return;
      dragging.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, [collapseThreshold, maxWidth, minWidth, side]);

  return { width: collapsed ? 0 : width, collapsed, onMouseDown, setCollapsed };
}
