# DiDaWork

轻量的桌面工作计时与上下班闹钟。驻留菜单栏，每分钟采样一次键鼠活动，把一天里的活跃时段记录下来，
用统计卡片呈现；同时提供多组上下班闹钟（复刻 macOS 闹钟的响铃与关闭方式）。

- **技术栈**：Tauri 2 + Rust · React 19 + TypeScript · Tailwind CSS v4 · shadcn 风格组件
- **数据全部本地**：`settings.json` + `alarms.json` + 按月归档的 `activity/YYYY-MM.json`
- **跨平台**：macOS / Windows / Linux 各有独立的键鼠检测实现，上层只依赖统一 trait

## 功能

| 模块 | 说明 |
| --- | --- |
| 统计 | 当天活跃总时长、近半月曲线（含日均）、近 7 天时间段、GitHub 风格年热力图；点任意方格/行可切换到那一天的详情 |
| 闹钟 | 上下班成对设置，可多组；标签、铃声、重复、启用/暂停；时间段不允许重叠（后端校验） |
| 响铃 | 到点弹**系统通知**（macOS / Linux 带「关闭闹钟」按钮）+ 系统铃声（最长可配置），不弹自建窗口；通知按钮、托盘菜单、主界面横幅、左右 Shift 任一都能关 |
| 设置 | 鼠标/键盘检测开关、最长静止时间、铃声时长、关闭快捷键、跳过规则、加班提醒、外观（浅色/深色/跟随系统）、中英文切换 |

## 架构

后端严格分层，依赖方向自上而下单向：

```
src-tauri/src/
├── commands/     前端唯一入口：整理参数、调用能力、广播事件（不含业务规则）
├── service/      编排层：采样线程、闹钟调度、响铃、托盘、窗口
├── domain/       纯逻辑：活跃状态机、闹钟模型与校验、统计聚合、设置（可单测，零 IO）
├── infra/        外部资源：JSON 存储（含内存缓存与原子写入）、ID 生成
├── platform/     平台兼容层：键鼠检测、修饰键监听、系统铃声、系统通知、打开链接
├── events.rs     前后端事件名契约
└── state.rs      进程内共享状态（细粒度锁）
```

为什么这样切分：

- **domain 不碰系统 API**，所以活动状态机、闹钟重叠校验、统计分级都能用普通单元测试覆盖（`cargo test`）。
- **platform 只暴露两个 trait**：`InputMonitor`（采样）与 `ShortcutWatcher`（修饰键），三套实现互换不影响上层。
- **commands 只做转译**，前端不会绕过领域规则。
- **统计聚合在后端完成**，前端拿到的就是可直接渲染的数组，WebView 不做逐日循环。

### 活跃判定

采样周期固定 1 分钟，每次只拿到"上一分钟内有没有输入"这一个布尔值，状态机负责还原时间段：

```
采样:  ●  ●  ●  ●  ○  ○  ○  ○  ○  ●  ●
状态: 活跃 ─────────────┐
           （冻结窗口）  │ 超过「最长静止时间」才判定离开
记录:  [───────────────]              [───
```

判定离开的时间点是**最后一次检测到键鼠输入**的时刻，而不是判定时刻，这样统计不会因为阈值而虚高。

### 平台差异

| 平台 | 活动检测 | 键鼠区分 | 关闭快捷键 | 通知按钮 |
| --- | --- | --- | --- | --- |
| macOS | `CGEventSourceCounterForEventType` 事件计数器 | ✅ 鼠标/键盘分开统计 | `CGEventSourceKeyState` 读左右键码 | ✅ `NSUserNotification.mainButton` |
| Windows | `GetLastInputInfo` 静止时长 | ❌ 只有聚合信号 | `GetAsyncKeyState` | ❌ 见下 |
| Linux | XScreenSaver 空闲时间 | ❌ 只有聚合信号 | `XQueryKeymap` 键位图 | ✅ freedesktop `actions` |

不区分鼠标与键盘的平台会把两个信号同时置位，因此单独关掉任一开关仍然正常工作；
X11 不可用（例如 Wayland）时自动降级为"不采集"，应用不会崩溃。

### 系统通知

闹钟到点只做两件事：循环播放铃声、发一条系统通知——**不创建任何自绘窗口**。
通知标题是闹钟名，正文是「该做什么 · 触发时刻」，在支持按钮的平台上还带一个
「关闭闹钟」，点了就停铃。实现集中在 `platform/notify.rs`：

| 平台 | 底层 | 按钮 | 说明 |
| --- | --- | --- | --- |
| macOS | `mac-notification-sys`（`NSUserNotification`） | ✅ | `mainButton` 是可点按钮。等待点击是阻塞调用，放在独立线程里；通知被系统收起时该线程自己结束 |
| Linux | `notify-rust`（freedesktop 通知） | ✅ | `action("dismiss", …)`；通知守护进程不支持 action 时按钮不显示，其它照常 |
| Windows | `tauri-plugin-notification`（WinRT toast） | ❌ | toast 按钮要自己拼 toast XML 并注册 AUMID，插件没暴露这层能力，因此**砍掉按钮**：停铃用托盘菜单「关闭闹钟」或左右 Shift |

#### 通知身份（macOS）

`NSUserNotification` 必须知道"这条通知是谁发的"。不主动设置时，`mac-notification-sys`
会先跑一句 AppleScript `get id of application "use_default"` 去猜，猜不到就由
LaunchServices 弹出「Where is use_default?」的选择应用窗口——**每次响铃都挡一次**。
所以应用启动时（`platform::notify::prepare`）先把身份定好，之后发通知不再问系统：

| 运行方式 | 身份 | 效果 |
| --- | --- | --- |
| 在 `.app` 包里（正常安装后打开） | `tauri.conf.json` 的 `identifier` | 通知显示为 DiDaWork，带应用图标 |
| 裸二进制（`npm run tauri dev`） | `com.apple.Terminal` | 系统把它当成 Terminal 发的（与 `tauri-plugin-notification` 的做法一致）；开发期没有 bundle id，这是唯一可靠的宿主身份 |

铃声到点（设置里的「铃声最长时间」）会自动收尾：声音停、会话清空、主界面横幅消失、
托盘菜单里的「关闭闹钟」变灰。通知本身则留在通知中心当记录，之后再点它的按钮也不会
误停后来那一次铃声（`ringer::dismiss_session` 用 `startedMs` 做过期校验）。

## 开发

```bash
npm install            # 前端依赖
npm run tauri dev      # 开发（Vite + Tauri）
npm run tauri build    # 打包
cargo test --manifest-path src-tauri/Cargo.toml   # 后端单元测试
npx tsc --noEmit       # 类型检查
```

前端目录：

```
src/
├── app/          数据总线、导航配置
├── components/   ui（shadcn 风格原子组件）/ layout / charts
├── features/     dashboard · alarms · settings（页面）· ring（响铃横幅）
├── hooks/        业务钩子（useTheme / useNow）
├── i18n/         中英文案与 Provider
└── lib/          api 封装、类型、时间与格式化工具
```

### 窗口外观

没有系统标题栏，内容铺满整窗：

- **macOS**：`titleBarStyle: overlay` + `hiddenTitle`，红黄绿交通灯浮在内容左上角，
  侧边栏顶部留出一条 44px 的拖拽区（`data-tauri-drag-region`）给它让位；
- **Windows / Linux**：启动时 `set_decorations(false)`，最小化/最大化/关闭由
  `components/layout/WindowControls.tsx` 自绘。

配色取自 otty 的 April / April Dark 主题（苔绿强调色），全部收敛到 `src/index.css`
的 CSS 变量里；明暗由 `<html data-theme>` 切换，`lib/theme.ts` 负责把设置里的
`system / light / dark` 解析成实际颜色（`system` 会订阅 `prefers-color-scheme` 变化），
并把结果缓存到 localStorage，保证下次启动首帧就是对的颜色。

### 数据文件

```jsonc
// activity/2026-11.json
{
  "2026-11-24": [
    { "startMs": 1795516800000, "endMs": 1795525200000 }
  ]
}
```

时段落盘前会排序并合并 60 秒以内的缝隙，文件里始终是"干净"的区间。
写入采用「临时文件 + 改名」，避免异常退出留下半截 JSON。

### 数据存放位置

所有数据都在系统的应用数据目录下（`app_data_dir()` + bundle identifier
`com.bugu.didawork`），关掉软件、重启系统都不会丢：

| 系统 | 目录 |
| --- | --- |
| macOS | `~/Library/Application Support/com.bugu.didawork/` |
| Windows | `%APPDATA%\com.bugu.didawork\`（即 `C:\Users\<用户>\AppData\Roaming\...`） |
| Linux | `~/.local/share/com.bugu.didawork/` |

目录内容：

```
settings.json            设置（含外观偏好）
alarms.json              闹钟列表
activity/2026-11.json    按月归档的活跃时段
```

写入时机：设置/闹钟一改就落盘；进行中的时段每 5 分钟打一次检查点，
退出应用时再 flush 一次。所以**关窗口只是收进菜单栏，数据早已在磁盘上**，
即使强制结束进程，最多丢 5 分钟。设置页的「数据目录」一行会直接显示当前路径。
