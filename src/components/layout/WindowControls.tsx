import type { ReactNode } from "react";
import { Minus, Square, X } from "lucide-react";

import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Windows / Linux 的自绘窗口按钮。
 *
 * macOS 用系统自带的红黄绿交通灯（透明标题栏叠加在内容上），其他平台把原生
 * 标题栏关掉后由前端画按钮，视觉上才和 macOS 一致——都不会出现灰色标题条。
 */
export function WindowControls() {
  const invoke = (action: "minimize" | "maximize" | "close") => {
    try {
      const window = getCurrentWindow();
      if (action === "minimize") void window.minimize();
      else if (action === "maximize") void window.toggleMaximize();
      else void window.close();
    } catch (error) {
      console.error("窗口操作失败", error);
    }
  };

  return (
    <div className="no-drag -mr-1 flex h-full items-stretch">
      <ControlButton label="最小化" onClick={() => invoke("minimize")}>
        <Minus className="size-3.5" strokeWidth={2} />
      </ControlButton>
      <ControlButton label="最大化" onClick={() => invoke("maximize")}>
        <Square className="size-3" strokeWidth={2} />
      </ControlButton>
      <ControlButton label="关闭" danger onClick={() => invoke("close")}>
        <X className="size-3.5" strokeWidth={2} />
      </ControlButton>
    </div>
  );
}

function ControlButton({
  label,
  danger = false,
  onClick,
  children,
}: {
  label: string;
  danger?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={
        danger
          ? "flex w-11 items-center justify-center text-ink-soft transition-colors hover:bg-danger hover:text-white"
          : "flex w-11 items-center justify-center text-ink-soft transition-colors hover:bg-hairline/70 hover:text-ink"
      }
    >
      {children}
    </button>
  );
}
