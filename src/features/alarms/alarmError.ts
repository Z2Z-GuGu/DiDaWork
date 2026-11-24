import { isValidationError, toMessage } from "@/lib/api";
import type { MessageKey } from "@/i18n/messages";

type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string;

/** 把后端返回的校验错误码翻译成当前语言的提示。 */
export function describeAlarmError(error: unknown, t: Translate): string {
  if (!isValidationError(error)) {
    return t("alarms.error.unknown", { message: toMessage(error) });
  }
  switch (error.code) {
    case "labelRequired":
      return t("alarms.error.labelRequired");
    case "rangeInvalid":
      return t("alarms.error.rangeInvalid");
    case "weekdaysRequired":
      return t("alarms.error.weekdaysRequired");
    case "weekdaysInvalid":
      return t("alarms.error.weekdaysInvalid");
    case "overlap":
      return t("alarms.error.overlap", { label: error.conflicts.join(" / ") });
    default:
      return t("alarms.error.unknown", { message: error.message });
  }
}
