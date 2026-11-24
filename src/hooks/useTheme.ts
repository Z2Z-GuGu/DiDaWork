import { useEffect } from "react";

import { applyTheme, resolveTheme, watchSystemTheme } from "@/lib/theme";
import type { Theme } from "@/lib/types";

/**
 * 把设置里的明暗偏好落到 `<html data-theme>`。
 *
 * 选「跟随系统」时会订阅系统外观变化，用户不改设置也能跟着 macOS/Windows 切换。
 */
export function useTheme(theme: Theme): void {
  useEffect(() => {
    applyTheme(resolveTheme(theme));
    if (theme !== "system") return;
    return watchSystemTheme(applyTheme);
  }, [theme]);
}
