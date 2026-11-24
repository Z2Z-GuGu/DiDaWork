import { ExternalLink, FolderOpen, Monitor, Moon, Power, Sun } from "lucide-react";

import { useAppData } from "@/app/AppDataProvider";
import { PageHeader } from "@/components/layout/PageHeader";
import { Row } from "@/components/layout/Row";
import { Section } from "@/components/layout/Section";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Segmented } from "@/components/ui/segmented";
import { Switch } from "@/components/ui/switch";
import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import type { Language, Shortcut, Theme } from "@/lib/types";

const FREEZE_OPTIONS = [1, 2, 3, 5, 10, 15, 20, 30];
const RING_OPTIONS = [15, 30, 45, 60, 90, 120];
const OVERTIME_OPTIONS = [5, 10, 15, 20, 30, 60];

/** 设置页：检测、闹钟、通用、关于四块。 */
export function SettingsPage() {
  const { t, lang } = useI18n();
  const { settings, info, updateSettings } = useAppData();

  return (
    <>
      <PageHeader title={t("settings.title")} />

      <div className="flex flex-col gap-3">
        <Section title={t("settings.detection")} hint={t("settings.detectionHint")}>
          <div className="divide-y divide-hairline">
            <Row title={t("settings.detectMouse")} hint={t("settings.detectMouseHint")}>
              <Switch
                checked={settings.detectMouse}
                onCheckedChange={(checked) => void updateSettings({ detectMouse: checked })}
              />
            </Row>
            <Row title={t("settings.detectKeyboard")} hint={t("settings.detectKeyboardHint")}>
              <Switch
                checked={settings.detectKeyboard}
                onCheckedChange={(checked) => void updateSettings({ detectKeyboard: checked })}
              />
            </Row>
            <Row title={t("settings.freeze")} hint={t("settings.freezeHint")}>
              <Select
                value={String(settings.freezeMinutes)}
                onValueChange={(value) =>
                  void updateSettings({ freezeMinutes: Number(value) })
                }
              >
                <SelectTrigger className="w-[104px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {FREEZE_OPTIONS.map((minutes) => (
                    <SelectItem key={minutes} value={String(minutes)}>
                      {minutes} {t("common.minute")}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Row>
          </div>
        </Section>

        <Section title={t("settings.alarm")}>
          <div className="divide-y divide-hairline">
            <Row title={t("settings.ringDuration")} hint={t("settings.ringDurationHint")}>
              <Select
                value={String(settings.ringMaxSeconds)}
                onValueChange={(value) =>
                  void updateSettings({ ringMaxSeconds: Number(value) })
                }
              >
                <SelectTrigger className="w-[104px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {RING_OPTIONS.map((seconds) => (
                    <SelectItem key={seconds} value={String(seconds)}>
                      {seconds} {t("common.second")}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Row>
            <Row title={t("settings.shortcut")} hint={t("settings.shortcutHint")}>
              <Select
                value={settings.dismissShortcut}
                onValueChange={(value) =>
                  void updateSettings({ dismissShortcut: value as Shortcut })
                }
              >
                <SelectTrigger className="w-[132px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="bothShift">{t("settings.shortcut.bothShift")}</SelectItem>
                  <SelectItem value="bothControl">
                    {t("settings.shortcut.bothControl")}
                  </SelectItem>
                  <SelectItem value="bothAlt">{t("settings.shortcut.bothAlt")}</SelectItem>
                </SelectContent>
              </Select>
            </Row>
            <Row title={t("settings.skipClockIn")} hint={t("settings.skipClockInHint")}>
              <Switch
                checked={settings.skipClockInWhenActive}
                onCheckedChange={(checked) =>
                  void updateSettings({ skipClockInWhenActive: checked })
                }
              />
            </Row>
            <Row title={t("settings.skipClockOut")} hint={t("settings.skipClockOutHint")}>
              <Switch
                checked={settings.skipClockOutWhenIdle}
                onCheckedChange={(checked) =>
                  void updateSettings({ skipClockOutWhenIdle: checked })
                }
              />
            </Row>
            <Row title={t("settings.overtime")} hint={t("settings.overtimeHint")}>
              <Switch
                checked={settings.overtimeReminder}
                onCheckedChange={(checked) => void updateSettings({ overtimeReminder: checked })}
              />
            </Row>
            <Row
              title={t("settings.overtimeInterval")}
              className={settings.overtimeReminder ? undefined : "opacity-45"}
            >
              <Select
                value={String(settings.overtimeIntervalMinutes)}
                onValueChange={(value) =>
                  void updateSettings({ overtimeIntervalMinutes: Number(value) })
                }
                disabled={!settings.overtimeReminder}
              >
                <SelectTrigger className="w-[104px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {OVERTIME_OPTIONS.map((minutes) => (
                    <SelectItem key={minutes} value={String(minutes)}>
                      {minutes} {t("common.minute")}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Row>
          </div>
        </Section>

        <Section title={t("settings.appearance")}>
          <div className="divide-y divide-hairline">
            <Row title={t("settings.theme")} hint={t("settings.appearanceHint")}>
              <Segmented<Theme>
                value={settings.theme}
                onChange={(theme) => void updateSettings({ theme })}
                options={[
                  {
                    value: "system",
                    label: t("settings.theme.system"),
                    icon: <Monitor className="size-3.5" />,
                  },
                  {
                    value: "light",
                    label: t("settings.theme.light"),
                    icon: <Sun className="size-3.5" />,
                  },
                  {
                    value: "dark",
                    label: t("settings.theme.dark"),
                    icon: <Moon className="size-3.5" />,
                  },
                ]}
              />
            </Row>
          </div>
        </Section>

        <Section title={t("settings.general")}>
          <div className="divide-y divide-hairline">
            <Row title={t("settings.language")}>
              <Select
                value={settings.language}
                onValueChange={(value) => void updateSettings({ language: value as Language })}
              >
                <SelectTrigger className="w-[132px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="system">{t("settings.language.system")}</SelectItem>
                  <SelectItem value="zh">{t("settings.language.zh")}</SelectItem>
                  <SelectItem value="en">{t("settings.language.en")}</SelectItem>
                </SelectContent>
              </Select>
            </Row>
          </div>
        </Section>

        <Section title={t("settings.about")}>
          <div className="divide-y divide-hairline">
            <Row title={t("app.name")}>
              <span className="tabular text-[12px] text-ink-soft">
                {t("settings.version")} {info?.version ?? "—"}
              </span>
            </Row>
            <Row title={t("settings.platform")}>
              <span className="text-[12px] text-ink-soft">{info?.platform ?? "—"}</span>
            </Row>
            <Row title={t("settings.repository")}>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => info && void openRepository(info.repository)}
              >
                <ExternalLink className="size-3.5" />
                {t("settings.openRepository")}
              </Button>
            </Row>
            <Row title={t("settings.dataDir")} hint={info?.dataDir ?? t("settings.dataDirHint")}>
              <Button variant="secondary" size="sm" onClick={() => void openDataDir()}>
                <FolderOpen className="size-3.5" />
                {t("settings.openDataDir")}
              </Button>
            </Row>
            <Row title={t("settings.quit")} hint={t("settings.quitHint")}>
              <Button variant="secondary" size="sm" onClick={() => void api.quitApp()}>
                <Power className="size-3.5" />
                {t("settings.quit")}
              </Button>
            </Row>
          </div>
        </Section>
      </div>

      <p className="mt-4 text-center text-[10px] text-ink-faint">
        {lang === "zh" ? "DiDaWork · 本地记录，不上传任何数据" : "DiDaWork · all data stays local"}
      </p>
    </>
  );
}

async function openDataDir() {
  try {
    await api.openDataDir();
  } catch (error) {
    console.error("打开数据目录失败", error);
  }
}

async function openRepository(url: string) {
  try {
    await api.openUrl(url);
  } catch (error) {
    console.error("打开仓库失败", error);
  }
}
