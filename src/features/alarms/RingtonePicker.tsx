import { useState } from "react";
import { Check, ChevronDown, FileText, Play, Plus } from "lucide-react";

import { useAppData } from "@/app/AppDataProvider";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { AddRingtoneDialog } from "./AddRingtoneDialog";

/**
 * 铃声选择。
 *
 * 列表项左边是名字（文件夹名），右边在有版权声明时放一个图标，
 * 点图标用系统默认应用打开对应的 txt；列表底部是"添加自定义铃声"。
 */
export function RingtonePicker({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (id: string) => void;
}) {
  const { t } = useI18n();
  const { ringtones, refreshRingtones } = useAppData();
  const [open, setOpen] = useState(false);
  const [adding, setAdding] = useState(false);

  const current = ringtones.find((item) => item.id === value);

  // 每次展开都重新扫盘：用户可能刚在访达里往目录中丢了一首歌。
  function handleOpenChange(next: boolean) {
    setOpen(next);
    if (next) void refreshRingtones().catch(() => undefined);
  }

  function preview() {
    if (!current) return;
    void api.previewRingtone(current.id).catch(() => undefined);
  }

  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-[11px] font-medium text-ink-soft">{label}</span>
      <div className="flex items-center gap-1.5">
        <Popover open={open} onOpenChange={handleOpenChange}>
          <PopoverTrigger asChild>
            <button
              type="button"
              className={cn(
                "flex h-8 w-[132px] items-center justify-between gap-1 rounded-lg border border-hairline-strong bg-surface px-2.5",
                "text-[13px] text-ink outline-none transition-colors hover:bg-subtle",
                "focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent/25",
              )}
            >
              <span className={cn("truncate", current ? "text-ink" : "text-ink-faint")}>
                {current?.name ?? t("alarms.ringtoneMissing")}
              </span>
              <ChevronDown className="size-3.5 shrink-0 text-ink-faint" />
            </button>
          </PopoverTrigger>
          <PopoverContent className="w-[248px]">
            <div className="max-h-60 overflow-y-auto">
              {ringtones.length === 0 ? (
                <p className="px-2 py-3 text-center text-[11px] text-ink-faint">
                  {t("alarms.ringtoneEmpty")}
                </p>
              ) : (
                ringtones.map((item) => (
                  <div
                    key={item.id}
                    className="flex items-center gap-0.5 rounded-md transition-colors hover:bg-accent-soft"
                  >
                    <button
                      type="button"
                      onClick={() => {
                        onChange(item.id);
                        setOpen(false);
                      }}
                      className={cn(
                        "flex h-7 min-w-0 flex-1 items-center gap-1.5 rounded-md px-2 text-left text-[13px]",
                        item.id === value ? "text-accent" : "text-ink",
                      )}
                    >
                      <span className="min-w-0 flex-1 truncate">{item.name}</span>
                      {item.id === value ? <Check className="size-3.5 shrink-0" /> : null}
                    </button>
                    {item.hasLicense ? (
                      <button
                        type="button"
                        title={t("alarms.viewLicense")}
                        onClick={() => void api.openRingtoneLicense(item.id).catch(() => undefined)}
                        className="mr-1 shrink-0 rounded p-1 text-ink-faint transition-colors hover:bg-hairline hover:text-accent"
                      >
                        <FileText className="size-3.5" />
                      </button>
                    ) : null}
                  </div>
                ))
              )}
            </div>

            <div className="my-1 h-px bg-hairline" />
            <button
              type="button"
              onClick={() => {
                setOpen(false);
                setAdding(true);
              }}
              className="flex h-7 w-full items-center gap-1.5 rounded-md px-2 text-left text-[13px] text-accent transition-colors hover:bg-accent-soft"
            >
              <Plus className="size-3.5" />
              {t("alarms.addRingtone")}
            </button>
          </PopoverContent>
        </Popover>

        <Button
          variant="ghost"
          size="icon"
          title={t("common.preview")}
          disabled={!current}
          onClick={preview}
        >
          <Play className="size-3.5" />
        </Button>
      </div>

      <AddRingtoneDialog
        open={adding}
        onOpenChange={setAdding}
        onAdded={(id) => onChange(id)}
      />
    </div>
  );
}
