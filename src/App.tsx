import { useState } from "react";
import { TooltipProvider } from "@/components/ui/tooltip";

import { AppDataProvider, useAppData } from "@/app/AppDataProvider";
import { type Tab } from "@/app/navigation";
import { Shell } from "@/components/layout/Shell";
import { AlarmsPage } from "@/features/alarms/AlarmsPage";
import { DashboardPage } from "@/features/dashboard/DashboardPage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { useTheme } from "@/hooks/useTheme";
import { I18nProvider } from "@/i18n";

/** 开发时可以用 `?tab=settings` 直接打开某一页，方便在浏览器里截图/调样式。 */
function initialTab(): Tab {
  if (!import.meta.env.DEV) return "stats";
  const value = new URLSearchParams(window.location.search).get("tab");
  return value === "alarms" || value === "settings" ? value : "stats";
}

export default function App() {
  return (
    <AppDataProvider>
      <AppContent />
    </AppDataProvider>
  );
}

function AppContent() {
  const { settings } = useAppData();
  const [tab, setTab] = useState<Tab>(initialTab);

  useTheme(settings.theme);

  return (
    <I18nProvider language={settings.language}>
      <TooltipProvider delayDuration={220} skipDelayDuration={400}>
        <Shell tab={tab} onTabChange={setTab}>
          {tab === "stats" ? <DashboardPage /> : null}
          {tab === "alarms" ? <AlarmsPage /> : null}
          {tab === "settings" ? <SettingsPage /> : null}
        </Shell>
      </TooltipProvider>
    </I18nProvider>
  );
}
