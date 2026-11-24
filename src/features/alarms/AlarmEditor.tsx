import { useEffect, useState } from "react";
import { ArrowRight, Play, Trash2 } from "lucide-react";

import { useAppData } from "@/app/AppDataProvider";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
} from "@/components/ui/dialog";
import { Input, TimeInput } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { weekdayShort } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { Alarm } from "@/lib/types";
import { describeAlarmError } from "./alarmError";

const PRESET_ALL = [true, true, true, true, true, true, true];
const PRESET_WORKDAYS = [true, true, true, true, true, false, false];
const PRESET_WEEKEND = [false, false, false, false, false, true, true];

export function emptyAlarm(ringtone: string): Alarm {
  return {
    id: "",
    label: "",
    clockIn: "09:00",
    clockOut: "18:00",
    ringtoneIn: ringtone,
    ringtoneOut: ringtone,
    weekdays: [...PRESET_WORKDAYS],
    enabled: true,
  };
}

/** 新建 / 编辑闹钟的弹窗。校验以后端为准，前端只做即时的空值提示。 */
export function AlarmEditor({
  open,
  alarm,
  onOpenChange,
}: {
  open: boolean;
  alarm: Alarm | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, lang } = useI18n();
  const { ringtones, saveAlarm, deleteAlarm } = useAppData();
  const [draft, setDraft] = useState<Alarm>(() => alarm ?? emptyAlarm(ringtones[0]?.id ?? "glass"));
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // 每次打开时同步到最新数据（编辑态 / 新建态）
  useEffect(() => {
    if (!open) return;
    setDraft(alarm ?? emptyAlarm(ringtones[0]?.id ?? "glass"));
    setError(null);
  }, [open, alarm, ringtones]);

  const patch = (changes: Partial<Alarm>) => setDraft((prev) => ({ ...prev, ...changes }));

  async function handleSave() {
    if (!draft.label.trim()) {
      setError(t("alarms.error.labelRequired"));
      return;
    }
    setSaving(true);
    try {
      await saveAlarm({ ...draft, label: draft.label.trim() });
      onOpenChange(false);
    } catch (err) {
      setError(describeAlarmError(err, t));
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete() {
    if (!draft.id) {
      onOpenChange(false);
      return;
    }
    try {
      await deleteAlarm(draft.id);
      onOpenChange(false);
    } catch (err) {
      setError(describeAlarmError(err, t));
    }
  }

  const ringtoneItems = ringtones.map((ringtone) => ({
    value: ringtone.id,
    label: lang === "zh" ? ringtone.nameZh : ringtone.nameEn,
  }));

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        label={draft.id ? t("alarms.editTitle") : t("alarms.newTitle")}
        aria-describedby={undefined}
      >
        <div className="mt-4 space-y-4">
          {/* 标签 */}
          <div className="space-y-1.5">
            <label className="text-[11px] font-medium text-ink-soft">{t("alarms.label")}</label>
            <Input
              autoFocus
              value={draft.label}
              placeholder={t("alarms.labelPlaceholder")}
              onChange={(event) => patch({ label: event.target.value })}
            />
          </div>

          {/* 上下班时间 */}
          <div className="flex items-end gap-3">
            <div className="space-y-1.5">
              <label className="text-[11px] font-medium text-ink-soft">
                {t("alarms.clockIn")}
              </label>
              <TimeInput
                value={draft.clockIn}
                onValueChange={(clockIn) => patch({ clockIn })}
                className="w-[96px]"
              />
            </div>
            <ArrowRight className="mb-2.5 size-4 text-ink-faint" />
            <div className="space-y-1.5">
              <label className="text-[11px] font-medium text-ink-soft">
                {t("alarms.clockOut")}
              </label>
              <TimeInput
                value={draft.clockOut}
                onValueChange={(clockOut) => patch({ clockOut })}
                className="w-[96px]"
              />
            </div>
          </div>

          {/* 重复 */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="text-[11px] font-medium text-ink-soft">
                {t("alarms.repeat")}
              </label>
              <div className="flex items-center gap-2.5 text-[11px] text-ink-faint">
                <PresetButton onClick={() => patch({ weekdays: [...PRESET_ALL] })}>
                  {t("alarms.repeatEveryday")}
                </PresetButton>
                <PresetButton onClick={() => patch({ weekdays: [...PRESET_WORKDAYS] })}>
                  {t("alarms.repeatWorkdays")}
                </PresetButton>
                <PresetButton onClick={() => patch({ weekdays: [...PRESET_WEEKEND] })}>
                  {t("alarms.repeatWeekend")}
                </PresetButton>
              </div>
            </div>
            <div className="flex gap-1.5">
              {draft.weekdays.map((active, index) => (
                <button
                  key={index}
                  type="button"
                  onClick={() =>
                    patch({
                      weekdays: draft.weekdays.map((value, cursor) =>
                        cursor === index ? !value : value,
                      ),
                    })
                  }
                  className={cn(
                    "h-7 flex-1 rounded-md border text-[11px] transition-colors",
                    active
                      ? "border-accent bg-accent text-on-accent"
                      : "border-hairline-strong text-ink-soft hover:bg-subtle",
                  )}
                >
                  {weekdayShort(index, lang)}
                </button>
              ))}
            </div>
          </div>

          {/* 铃声 */}
          <div className="space-y-2">
            <RingtoneRow
              label={t("alarms.ringtoneIn")}
              value={draft.ringtoneIn}
              items={ringtoneItems}
              onChange={(value) => patch({ ringtoneIn: value })}
            />
            <RingtoneRow
              label={t("alarms.ringtoneOut")}
              value={draft.ringtoneOut}
              items={ringtoneItems}
              onChange={(value) => patch({ ringtoneOut: value })}
            />
          </div>

          {/* 启用 */}
          <div className="flex items-center justify-between rounded-lg bg-subtle px-3 py-2">
            <span className="text-[13px] text-ink">{t("alarms.enable")}</span>
            <Switch
              checked={draft.enabled}
              onCheckedChange={(checked) => patch({ enabled: checked })}
            />
          </div>

          {error ? <p className="text-[11px] text-danger">{error}</p> : null}

          <div className="flex items-center justify-between pt-1">
            {draft.id ? (
              <Button variant="danger" size="md" onClick={handleDelete}>
                <Trash2 className="size-3.5" />
                {t("common.delete")}
              </Button>
            ) : (
              <span />
            )}
            <div className="flex items-center gap-2">
              <Button variant="ghost" onClick={() => onOpenChange(false)}>
                {t("common.cancel")}
              </Button>
              <Button variant="primary" onClick={handleSave} disabled={saving}>
                {t("common.save")}
              </Button>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function PresetButton({ onClick, children }: { onClick: () => void; children: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="rounded px-1 text-accent transition-colors hover:bg-accent-soft"
    >
      {children}
    </button>
  );
}

function RingtoneRow({
  label,
  value,
  items,
  onChange,
}: {
  label: string;
  value: string;
  items: { value: string; label: string }[];
  onChange: (value: string) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-[11px] font-medium text-ink-soft">{label}</span>
      <div className="flex items-center gap-1.5">
        <Select value={value} onValueChange={onChange}>
          <SelectTrigger className="w-[132px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {items.map((item) => (
              <SelectItem key={item.value} value={item.value}>
                {item.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          variant="ghost"
          size="icon"
          title={t("common.preview")}
          onClick={() => void api.previewRingtone(value)}
        >
          <Play className="size-3.5" />
        </Button>
      </div>
    </div>
  );
}
