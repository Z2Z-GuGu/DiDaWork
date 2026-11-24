import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { BellRing, CircleCheck } from "lucide-react";

import { useI18n } from "@/i18n";
import { api } from "@/lib/api";
import type { RingInfo } from "@/lib/types";
import { Button } from "@/components/ui/button";

const IS_TAURI = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * 响铃横幅。
 *
 * 闹钟提示本身交给系统通知中心；这条横幅只在主窗口正好开着时出现，
 * 给一个更好点的"关闭闹钟"入口——顺便覆盖通知没有按钮的平台（Windows）。
 */
export function RingBanner() {
  const { t } = useI18n();
  const [info, setInfo] = useState<RingInfo | null>(null);

  useEffect(() => {
    if (!IS_TAURI) return;
    const disposers: Array<() => void> = [];
    let disposed = false;
    const track = (promise: Promise<() => void>) =>
      promise.then((dispose) => {
        if (disposed) dispose();
        else disposers.push(dispose);
      });

    track(listen<RingInfo>("ring-start", (event) => setInfo(event.payload)));
    track(listen("ring-end", () => setInfo(null)));

    return () => {
      disposed = true;
      disposers.forEach((dispose) => dispose());
    };
  }, []);

  if (!info) return null;

  return (
    <div className="mb-1 flex items-center gap-3 rounded-xl border border-accent/35 bg-accent-soft px-3 py-2">
      <span className="relative flex size-6 shrink-0 items-center justify-center">
        <span className="absolute inset-0 animate-ring-pulse rounded-full bg-accent/20" />
        <BellRing className="relative size-3.5 text-accent" strokeWidth={2.2} />
      </span>

      <div className="min-w-0 flex-1 leading-tight">
        <div className="truncate text-[12px] font-medium text-ink">
          {t("ring.ringing")} · {info.label}
        </div>
        <div className="tabular truncate text-[11px] text-ink-soft">
          {info.clock}
        </div>
      </div>

      <Button size="sm" variant="primary" onClick={() => void api.dismissRing()}>
        <CircleCheck className="size-3.5" />
        {t("ring.dismiss")}
      </Button>
    </div>
  );
}
