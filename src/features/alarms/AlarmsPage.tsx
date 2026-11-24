import { useState } from "react";
import { AlarmClock, ArrowRight, Pencil, Plus, Trash2 } from "lucide-react";

import { useAppData } from "@/app/AppDataProvider";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
} from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { useI18n } from "@/i18n";
import { cn } from "@/lib/utils";
import { weekdayShort } from "@/lib/format";
import type { Alarm } from "@/lib/types";
import { AlarmEditor } from "./AlarmEditor";
import { describeAlarmError } from "./alarmError";

/** 闹钟列表：上下班成对的长卡片，底部是添加入口。 */
export function AlarmsPage() {
  const { t } = useI18n();
  const { alarms, toggleAlarm, deleteAlarm } = useAppData();
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<Alarm | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Alarm | null>(null);
  const [error, setError] = useState<string | null>(null);

  function openEditor(alarm: Alarm | null) {
    setEditing(alarm);
    setEditorOpen(true);
    setError(null);
  }

  async function handleToggle(alarm: Alarm, enabled: boolean) {
    try {
      await toggleAlarm(alarm.id, enabled);
      setError(null);
    } catch (err) {
      setError(describeAlarmError(err, t));
    }
  }

  async function confirmDelete() {
    if (!pendingDelete) return;
    await deleteAlarm(pendingDelete.id);
    setPendingDelete(null);
  }

  return (
    <>
      <PageHeader
        title={t("alarms.title")}
        subtitle={t("alarms.subtitle")}
        actions={
          <Button variant="primary" size="sm" onClick={() => openEditor(null)}>
            <Plus className="size-3.5" />
            {t("alarms.add")}
          </Button>
        }
      />

      {error ? (
        <div className="mb-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-2 text-[12px] text-danger">
          {error}
        </div>
      ) : null}

      {alarms.length === 0 ? (
        <EmptyState onAdd={() => openEditor(null)} />
      ) : (
        <div className="flex flex-col gap-2.5">
          {alarms.map((alarm) => (
            <AlarmCard
              key={alarm.id}
              alarm={alarm}
              onToggle={(enabled) => void handleToggle(alarm, enabled)}
              onEdit={() => openEditor(alarm)}
              onDelete={() => setPendingDelete(alarm)}
            />
          ))}
          <button
            type="button"
            onClick={() => openEditor(null)}
            className="flex h-[46px] items-center justify-center gap-1.5 rounded-card border border-dashed border-hairline-strong text-[12px] text-ink-faint transition-colors hover:border-accent/50 hover:text-accent"
          >
            <Plus className="size-3.5" />
            {t("alarms.add")}
          </button>
        </div>
      )}

      <AlarmEditor open={editorOpen} alarm={editing} onOpenChange={setEditorOpen} />

      <Dialog open={Boolean(pendingDelete)} onOpenChange={(open) => !open && setPendingDelete(null)}>
        <DialogContent label={t("alarms.deleteTitle")} aria-describedby={undefined}>
          <p className="mt-2 text-[12px] leading-relaxed text-ink-soft">
            {t("alarms.deleteHint", { label: pendingDelete?.label ?? "" })}
          </p>
          <div className="mt-5 flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setPendingDelete(null)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              className="bg-danger hover:bg-danger/90"
              onClick={() => void confirmDelete()}
            >
              {t("common.delete")}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

function AlarmCard({
  alarm,
  onToggle,
  onEdit,
  onDelete,
}: {
  alarm: Alarm;
  onToggle: (enabled: boolean) => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const { t, lang } = useI18n();

  return (
    <div
      className={cn(
        "group flex items-center justify-between gap-4 rounded-card border border-hairline bg-surface px-4 py-3.5 shadow-card transition-opacity",
        !alarm.enabled && "opacity-55",
      )}
    >
      <div className="flex min-w-0 items-center gap-3.5">
        <div className="flex shrink-0 items-center gap-1.5">
          <span className="tabular text-[24px] font-semibold leading-none tracking-tight text-ink">
            {alarm.clockIn}
          </span>
          <ArrowRight className="size-3.5 translate-y-[1px] text-ink-faint" />
          <span className="tabular text-[24px] font-semibold leading-none tracking-tight text-ink-soft">
            {alarm.clockOut}
          </span>
        </div>

        <span className="h-8 w-px shrink-0 bg-hairline" />

        <div className="min-w-0">
          <div className="flex items-center gap-1.5">
            <span className="truncate text-[13px] font-medium text-ink">{alarm.label}</span>
            {!alarm.enabled ? (
              <span className="shrink-0 rounded bg-hairline px-1 py-[1px] text-[10px] text-ink-faint">
                {t("alarms.disabledHint")}
              </span>
            ) : null}
          </div>
          <div className="mt-0.5 truncate text-[11px] text-ink-faint">
            {repeatText(alarm, lang, t)}
          </div>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-1">
        <Switch checked={alarm.enabled} onCheckedChange={onToggle} />
        <Button variant="ghost" size="icon" onClick={onEdit} title={t("common.edit")}>
          <Pencil className="size-3.5" />
        </Button>
        <Button variant="ghost" size="icon" onClick={onDelete} title={t("common.delete")}>
          <Trash2 className="size-3.5" />
        </Button>
      </div>
    </div>
  );
}

function repeatText(
  alarm: Alarm,
  lang: "zh" | "en",
  t: (key: "alarms.repeatEveryday" | "alarms.repeatWorkdays" | "alarms.repeatWeekend") => string,
): string {
  const days = alarm.weekdays;
  if (days.every(Boolean)) return t("alarms.repeatEveryday");
  if (days.slice(0, 5).every(Boolean) && !days[5] && !days[6]) {
    return t("alarms.repeatWorkdays");
  }
  if (!days.slice(0, 5).some(Boolean) && days[5] && days[6]) {
    return t("alarms.repeatWeekend");
  }
  return days
    .map((active, index) => (active ? weekdayShort(index, lang) : null))
    .filter(Boolean)
    .join("、");
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  const { t } = useI18n();
  return (
    <div className="flex flex-col items-center justify-center rounded-card border border-hairline bg-surface px-6 py-14 text-center shadow-card">
      <div className="flex size-11 items-center justify-center rounded-full bg-accent-soft">
        <AlarmClock className="size-5 text-accent" />
      </div>
      <p className="mt-3 text-[13px] font-medium text-ink">{t("alarms.empty")}</p>
      <p className="mt-1 max-w-[280px] text-[11px] leading-relaxed text-ink-faint">
        {t("alarms.emptyHint")}
      </p>
      <Button variant="primary" className="mt-4" onClick={onAdd}>
        <Plus className="size-3.5" />
        {t("alarms.add")}
      </Button>
    </div>
  );
}
