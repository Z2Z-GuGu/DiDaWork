import { createContext, useContext, useMemo, type ReactNode } from "react";

import type { Lang } from "@/lib/format";
import type { Language } from "@/lib/types";
import { messages, type MessageKey } from "./messages";

interface I18nValue {
  lang: Lang;
  t: (key: MessageKey, vars?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nValue | null>(null);

/** `system` 交给浏览器/系统语言决定 */
export function resolveLang(language: Language): Lang {
  if (language === "zh" || language === "en") return language;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export function I18nProvider({
  language,
  children,
}: {
  language: Language;
  children: ReactNode;
}) {
  const value = useMemo<I18nValue>(() => {
    const lang = resolveLang(language);
    const dict = messages[lang] as Record<string, string>;
    const t = (key: MessageKey, vars?: Record<string, string | number>) => {
      let text = dict[key] ?? key;
      if (vars) {
        for (const [name, replacement] of Object.entries(vars)) {
          text = text.split(`{${name}}`).join(String(replacement));
        }
      }
      return text;
    };
    return { lang, t };
  }, [language]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  const context = useContext(I18nContext);
  if (!context) throw new Error("useI18n 必须在 I18nProvider 内使用");
  return context;
}
