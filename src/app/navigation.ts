import type { LucideIcon } from "lucide-react";
import { AlarmClock, BarChart3, Settings2 } from "lucide-react";

export type Tab = "stats" | "alarms" | "settings";

export interface NavItem {
  key: Tab;
  icon: LucideIcon;
  labelKey: "nav.stats" | "nav.alarms" | "nav.settings";
}

export const NAV_ITEMS: NavItem[] = [
  { key: "stats", icon: BarChart3, labelKey: "nav.stats" },
  { key: "alarms", icon: AlarmClock, labelKey: "nav.alarms" },
  { key: "settings", icon: Settings2, labelKey: "nav.settings" },
];
