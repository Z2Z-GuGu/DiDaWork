import { useEffect, useState } from "react";

/**
 * 返回一个按固定间隔刷新的"当前时间"。
 * 用于"已持续 xx"这类只需要粗粒度的展示，避免每帧重渲染。
 */
export function useNow(intervalMs = 30_000): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(timer);
  }, [intervalMs]);

  return now;
}
