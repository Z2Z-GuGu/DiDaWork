import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";

import { api } from "@/lib/api";
import type {
  AddedRingtone,
  Alarm,
  AppInfo,
  LiveStatus,
  Ringtone,
  RingtoneInput,
  Settings,
} from "@/lib/types";

/**
 * 应用数据总线。
 *
 * 只把"全局共享、会被多处读取"的数据放在这里（设置、闹钟、实时状态、铃声表），
 * 页面自己的数据（趋势、热力图…）由页面按需拉取，避免无谓的全量刷新。
 *
 * 后端会主动推送这几类变化，因此窗口之间（主窗口 / 闹钟浮窗）能保持一致。
 */

interface AppDataValue {
  ready: boolean;
  settings: Settings;
  alarms: Alarm[];
  live: LiveStatus;
  ringtones: Ringtone[];
  info: AppInfo | null;
  /** 每次后端采样后自增，供页面做"该刷新了"的依赖 */
  sampleTick: number;
  updateSettings: (patch: Partial<Settings>) => Promise<void>;
  saveAlarm: (alarm: Alarm) => Promise<Alarm[]>;
  deleteAlarm: (id: string) => Promise<void>;
  toggleAlarm: (id: string, enabled: boolean) => Promise<void>;
  /** 重新扫描铃声目录（用户可能在访达里直接增删了文件夹） */
  refreshRingtones: () => Promise<Ringtone[]>;
  /** 导入一首自定义铃声，成功后列表自动刷新 */
  addRingtone: (input: RingtoneInput) => Promise<AddedRingtone>;
}

const FALLBACK_SETTINGS: Settings = {
  detectMouse: true,
  detectKeyboard: true,
  freezeMinutes: 5,
  ringMaxSeconds: 60,
  dismissShortcut: "bothShift",
  skipClockInWhenActive: true,
  skipClockOutWhenIdle: true,
  overtimeReminder: true,
  overtimeIntervalMinutes: 10,
  language: "zh",
  theme: "system",
};

const AppDataContext = createContext<AppDataValue | null>(null);

export function AppDataProvider({ children }: { children: ReactNode }) {
  const [ready, setReady] = useState(false);
  const [settings, setSettings] = useState<Settings>(FALLBACK_SETTINGS);
  const [alarms, setAlarms] = useState<Alarm[]>([]);
  const [ringtones, setRingtones] = useState<Ringtone[]>([]);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [live, setLive] = useState<LiveStatus>({
    isActive: false,
    segmentStartMs: null,
    todayActiveMs: 0,
    lastSampleMs: 0,
    lastInputMs: null,
  });
  const [sampleTick, setSampleTick] = useState(0);

  // 用 ref 保存最新设置，订阅回调里可用，又不用重建监听。
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      try {
        const [nextSettings, nextAlarms, nextLive, nextRingtones, nextInfo] = await Promise.all([
          api.getSettings(),
          api.listAlarms(),
          api.getLiveStatus(),
          api.listRingtones(),
          api.getAppInfo(),
        ]);
        if (cancelled) return;
        setSettings(nextSettings);
        setAlarms(nextAlarms);
        setLive(nextLive);
        setRingtones(nextRingtones);
        setInfo(nextInfo);
      } catch (error) {
        console.error("初始化失败", error);
      } finally {
        if (!cancelled) setReady(true);
      }
    }

    void bootstrap();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const disposers: Array<() => void> = [];
    let disposed = false;

    const unlisten = (promise: Promise<() => void>) =>
      promise.then((dispose) => {
        if (disposed) dispose();
        else disposers.push(dispose);
      });

    unlisten(
      listen<LiveStatus>("live-status", (event) => {
        setLive(event.payload);
        setSampleTick((tick) => tick + 1);
      }),
    );
    unlisten(
      listen<Settings>("settings-changed", (event) => setSettings(event.payload)),
    );
    unlisten(listen<Alarm[]>("alarms-changed", (event) => setAlarms(event.payload)));

    return () => {
      disposed = true;
      disposers.forEach((dispose) => dispose());
    };
  }, []);

  const updateSettings = useCallback(async (patch: Partial<Settings>) => {
    const previous = settingsRef.current;
    const next = { ...previous, ...patch };
    setSettings(next); // 乐观更新，交互立即有反馈
    try {
      const saved = await api.saveSettings(next);
      setSettings(saved);
    } catch (error) {
      setSettings(previous);
      console.error("保存设置失败", error);
    }
  }, []);

  const saveAlarm = useCallback(async (alarm: Alarm) => {
    const list = await api.saveAlarm(alarm);
    setAlarms(list);
    return list;
  }, []);

  const deleteAlarm = useCallback(async (id: string) => {
    const list = await api.deleteAlarm(id);
    setAlarms(list);
  }, []);

  const toggleAlarm = useCallback(async (id: string, enabled: boolean) => {
    const list = await api.setAlarmEnabled(id, enabled);
    setAlarms(list);
  }, []);

  const refreshRingtones = useCallback(async () => {
    const list = await api.listRingtones();
    setRingtones(list);
    return list;
  }, []);

  const addRingtone = useCallback(async (input: RingtoneInput) => {
    const added = await api.addRingtone(input);
    setRingtones(added.ringtones);
    return added;
  }, []);

  const value = useMemo<AppDataValue>(
    () => ({
      ready,
      settings,
      alarms,
      live,
      ringtones,
      info,
      sampleTick,
      updateSettings,
      saveAlarm,
      deleteAlarm,
      toggleAlarm,
      refreshRingtones,
      addRingtone,
    }),
    [
      ready,
      settings,
      alarms,
      live,
      ringtones,
      info,
      sampleTick,
      updateSettings,
      saveAlarm,
      deleteAlarm,
      toggleAlarm,
      refreshRingtones,
      addRingtone,
    ],
  );

  return <AppDataContext.Provider value={value}>{children}</AppDataContext.Provider>;
}

export function useAppData(): AppDataValue {
  const context = useContext(AppDataContext);
  if (!context) throw new Error("useAppData 必须在 AppDataProvider 内使用");
  return context;
}
