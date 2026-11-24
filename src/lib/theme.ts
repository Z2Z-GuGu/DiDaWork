/**
 * 明暗主题。
 *
 * 设置里有 `system` / `light` / `dark` 三种偏好，这里统一解析成真正生效的两种颜色。
 * 解析结果写在 `<html data-theme>` 上，CSS 变量随之切换（见 index.css），
 * 同时缓存到 localStorage，让下一次启动能在首帧之前就把颜色定好，不会白屏闪一下。
 */

import type { Theme } from "@/lib/types";

export type ResolvedTheme = "light" | "dark";

const CACHE_KEY = "didawork.theme";
const DARK_QUERY = "(prefers-color-scheme: dark)";

export function systemTheme(): ResolvedTheme {
  return window.matchMedia(DARK_QUERY).matches ? "dark" : "light";
}

export function resolveTheme(theme: Theme): ResolvedTheme {
  return theme === "system" ? systemTheme() : theme;
}

/** 把解析后的主题应用到文档，并记下缓存。 */
export function applyTheme(theme: ResolvedTheme): void {
  const root = document.documentElement;
  if (root.dataset.theme !== theme) root.dataset.theme = theme;
  // 让原生控件（滚动条、原生 select、输入光标）也用对应配色
  root.style.colorScheme = theme;
  try {
    localStorage.setItem(CACHE_KEY, theme);
  } catch {
    // 隐私模式等场景下 localStorage 不可用，纯样式方案不受影响
  }
}

/** 启动时用：拿到上一次的主题，避免初始渲染和设置不一致导致的闪烁。 */
export function cachedTheme(): ResolvedTheme {
  try {
    const value = localStorage.getItem(CACHE_KEY);
    if (value === "light" || value === "dark") return value;
  } catch {
    // 忽略
  }
  return systemTheme();
}

/** 监听系统外观切换，返回取消订阅函数。 */
export function watchSystemTheme(handler: (theme: ResolvedTheme) => void): () => void {
  const query = window.matchMedia(DARK_QUERY);
  const listener = (event: MediaQueryListEvent) => handler(event.matches ? "dark" : "light");
  query.addEventListener("change", listener);
  return () => query.removeEventListener("change", listener);
}
