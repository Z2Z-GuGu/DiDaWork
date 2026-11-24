import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FileText, FolderOpen } from "lucide-react";

import { useAppData } from "@/app/AppDataProvider";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useI18n } from "@/i18n";
import type { MessageKey } from "@/i18n/messages";
import { toMessage } from "@/lib/api";

/**
 * 添加自定义铃声。
 *
 * 落地成 `<数据目录>/ringtones/<音乐名>/` 下一个音乐文件（必须有）+ 一个 txt（可选），
 * 跟内置铃声的结构完全一样——用户之后也能直接在访达里换掉文件。
 */
export function AddRingtoneDialog({
  open: isOpen,
  onOpenChange,
  onAdded,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdded: (id: string) => void;
}) {
  const { t } = useI18n();
  const { addRingtone } = useAppData();
  const [name, setName] = useState("");
  const [audioPath, setAudioPath] = useState<string | null>(null);
  const [licensePath, setLicensePath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // 每次打开都是干净的一张表
  useEffect(() => {
    if (!isOpen) return;
    setName("");
    setAudioPath(null);
    setLicensePath(null);
    setError(null);
    setSaving(false);
  }, [isOpen]);

  async function pickAudio() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: t("alarms.pickAudio"),
      filters: [{ name: t("alarms.pickAudioFilter"), extensions: ["m4a", "mp3"] }],
    });
    if (typeof picked === "string") {
      setAudioPath(picked);
      setError(null);
    }
  }

  async function pickLicense() {
    const picked = await open({
      multiple: false,
      directory: false,
      title: t("alarms.pickLicense"),
      filters: [{ name: t("alarms.pickLicenseFilter"), extensions: ["txt"] }],
    });
    if (typeof picked === "string") {
      setLicensePath(picked);
      setError(null);
    }
  }

  async function handleSave() {
    if (!name.trim()) {
      setError(t("alarms.error.ringtoneNameRequired"));
      return;
    }
    if (!audioPath) {
      setError(t("alarms.error.ringtoneAudioRequired"));
      return;
    }
    setSaving(true);
    try {
      const added = await addRingtone({ name, audioPath, licensePath });
      onAdded(added.created.id);
      onOpenChange(false);
    } catch (err) {
      setError(describeRingtoneError(err, t));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent
        label={t("alarms.addRingtoneTitle")}
        aria-describedby={undefined}
        className="w-[400px]"
      >
        <div className="mt-4 space-y-4">
          <div className="space-y-1.5">
            <label className="text-[11px] font-medium text-ink-soft">
              {t("alarms.ringtoneName")}
            </label>
            <Input
              autoFocus
              value={name}
              placeholder={t("alarms.ringtoneNamePlaceholder")}
              onChange={(event) => setName(event.target.value)}
            />
          </div>

          <FileField
            label={t("alarms.ringtoneAudio")}
            required
            path={audioPath}
            empty={t("alarms.noFile")}
            action={t("alarms.chooseFile")}
            onPick={pickAudio}
            onClear={() => setAudioPath(null)}
          />

          <FileField
            label={t("alarms.ringtoneLicense")}
            hint={t("alarms.optional")}
            path={licensePath}
            empty={t("alarms.noFile")}
            action={t("alarms.chooseFile")}
            onPick={pickLicense}
            onClear={() => setLicensePath(null)}
          />

          {error ? <p className="text-[11px] text-danger">{error}</p> : null}

          <div className="flex items-center justify-end gap-2 pt-1">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="primary" onClick={handleSave} disabled={saving}>
              {t("common.add")}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

/** 一行"选文件"控件：选了显示文件名，没选显示占位文案。 */
function FileField({
  label,
  hint,
  required,
  path,
  empty,
  action,
  onPick,
  onClear,
}: {
  label: string;
  hint?: string;
  required?: boolean;
  path: string | null;
  empty: string;
  action: string;
  onPick: () => void;
  onClear: () => void;
}) {
  const fileName = path ? path.split(/[/\\]/).pop() : null;
  return (
    <div className="space-y-1.5">
      <div className="flex items-baseline gap-1.5">
        <label className="text-[11px] font-medium text-ink-soft">{label}</label>
        {required ? <span className="text-[10px] text-danger">*</span> : null}
        {hint ? <span className="text-[10px] text-ink-faint">{hint}</span> : null}
      </div>
      <div className="flex items-center gap-2">
        <div className="flex h-8 min-w-0 flex-1 items-center gap-1.5 rounded-lg border border-hairline-strong bg-surface px-2.5">
          {fileName ? (
            <FileText className="size-3.5 shrink-0 text-ink-faint" />
          ) : null}
          <span
            className={`truncate text-[12px] ${fileName ? "text-ink" : "text-ink-faint"}`}
            title={path ?? undefined}
          >
            {fileName ?? empty}
          </span>
        </div>
        {fileName ? (
          <button
            type="button"
            onClick={onClear}
            className="shrink-0 rounded px-1 text-[11px] text-ink-faint transition-colors hover:text-danger"
          >
            ✕
          </button>
        ) : null}
        <Button variant="secondary" size="sm" onClick={onPick}>
          <FolderOpen className="size-3.5" />
          {action}
        </Button>
      </div>
    </div>
  );
}

/** 后端返回 `{ code, message }`，能识别的 code 用本地文案，其余兜底显示 message。 */
function describeRingtoneError(
  error: unknown,
  t: (key: MessageKey, vars?: Record<string, string | number>) => string,
): string {
  switch ((error as { code?: string } | null)?.code) {
    case "nameInvalid":
      return t("alarms.error.ringtoneNameInvalid");
    case "nameTaken":
      return t("alarms.error.ringtoneNameTaken");
    case "audioInvalid":
      return t("alarms.error.ringtoneAudioInvalid");
    case "licenseInvalid":
      return t("alarms.error.ringtoneLicenseInvalid");
    default:
      return t("alarms.error.ringtoneUnknown", { message: toMessage(error) });
  }
}
