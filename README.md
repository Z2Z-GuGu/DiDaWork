# DiDaWork

轻量的桌面工作计时与上下班闹钟。驻留菜单栏，每分钟统计一次键鼠活动，把一天里的活跃时段记录下来，
用统计卡片呈现；同时提供多组上下班闹钟（复刻 macOS 闹钟的响铃与关闭方式）。

- **技术栈**：Tauri 2 + Rust · React 19 + TypeScript · Tailwind CSS v4 · shadcn 风格组件
- **数据全部本地**：`settings.json` + `alarms.json` + 按月归档的 `activity/YYYY-MM.json`
- **跨平台**：macOS / Windows / Linux 各有独立的键鼠检测实现，上层只依赖统一 trait

## 功能

| 模块 | 说明 |
| --- | --- |
| 统计 | 当天活跃总时长、近半月曲线（含日均）、近 7 天时间段、GitHub 风格年热力图；点任意方格/行可切换到那一天的详情 |
| 闹钟 | 上下班成对设置，可多组；标签、铃声、重复、启用/暂停；时间段不允许重叠（后端校验）；加班提醒只在下班到下次上班之间生效，停手即结束 |
| 响铃 | 到点弹**系统通知**（macOS / Linux 带「关闭闹钟」按钮）+ 播放铃声库里的音乐（最长可配置），不弹自建窗口；通知按钮、托盘菜单、主界面横幅、左右 Shift 任一都能关 |
| 设置 | 鼠标/键盘检测开关、最长静止时间、铃声时长、关闭快捷键、跳过规则、加班提醒、外观（浅色/深色/跟随系统）、中英文切换 |

## 架构

后端严格分层，依赖方向自上而下单向：

```
src-tauri/src/
├── commands/     前端唯一入口：整理参数、调用能力、广播事件（不含业务规则）
├── service/      编排层：采样线程、闹钟调度、响铃、托盘、窗口
├── domain/       纯逻辑：活跃状态机、闹钟模型与校验、统计聚合、设置（可单测，零 IO）
├── infra/        外部资源：JSON 存储（含内存缓存与原子写入）、铃声库扫描与导入、ID 生成
├── platform/     平台兼容层：键鼠检测、修饰键监听、音乐播放、系统通知、打开链接
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

### 加班提醒

「下班后继续工作时提醒」不是"下班时间之后一活跃就提醒"，它有两个边界：

1. **空间边界**：只从本组闹钟的下班时刻，到**下一组闹钟的上班时刻**为止。上午 09:00–12:00、
   下午 14:00–18:00 时，上午那组只会在 `[12:00, 14:00)` 里提醒，14:00 之后交给下午那组，
   不会再喊"已经下班，注意休息"；没有下一组闹钟就一直管到当天结束。
2. **生命周期**：下班后先看到你在工作（进入加班状态），随后又看到你停下来（超过「最长静止时间」
   没有输入），这次加班就算结束了——当天再检测到活跃也不会复活。所以 12:10 停手、13:45 提前
   开工，不会再收到上午的提醒。

两个边界都落在 `domain/alarm.rs`（`overtime_window_end` / `overtime_due`）与
`service/alarm_scheduler.rs`（`DayLog` 记录"加班中 / 已结束"），跨天自动清空。

### 平台差异

| 平台 | 活动检测 | 键鼠区分 | 关闭快捷键 | 通知按钮 |
| --- | --- | --- | --- | --- |
| macOS | `CGEventSourceCounterForEventType` 事件计数器 | ✅ 鼠标/键盘分开统计 | `CGEventSourceKeyState` 读左右键码 | ✅ 通知分类里的 `UNNotificationAction` |
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
| macOS | `UNUserNotificationCenter`（UserNotifications） | ✅ | 通知分类（category）里的 `UNNotificationAction`；`willPresent` 返回「横幅 + 通知中心」，应用在前台也照弹 |
| Linux | `notify-rust`（freedesktop 通知） | ✅ | `action("dismiss", …)`；通知守护进程不支持 action 时按钮不显示，其它照常 |
| Windows | `tauri-plugin-notification`（WinRT toast） | ❌ | toast 按钮要自己拼 toast XML 并注册 AUMID，插件没暴露这层能力，因此**砍掉按钮**：停铃用托盘菜单「关闭闹钟」或左右 Shift |

#### 为什么不用 `NSUserNotification`

最早的实现基于 `NSUserNotification`（`mac-notification-sys`）。它在 macOS 11 起就被
标记弃用，现在系统上交付很不稳定：同一份代码会出现「这次弹、下次不弹」，而且它没有
授权概念，通知到底发不发取决于系统心情。换成 `UserNotifications` 后有三个确定的行为：

1. **授权**：启动时 `requestAuthorizationWithOptions:` 申请一次，系统会问「允许 / 不允许」；
   未授权时日志会打 `[notify] 通知未获授权，闹钟将只有铃声`，`addNotificationRequest`
   也会以 `UNErrorDomain error 1` 失败——而不是静默消失。
2. **前台也弹**：默认应用在前台时系统不显示横幅，`willPresent` 里返回
   `Banner | List` 强制弹出。
3. **可撤下**：响铃结束时按 identifier `removeDeliveredNotificationsWithIdentifiers:` 收掉，
   通知中心里不会留一条过期的提醒。

#### 应用必须签名（macOS）

`UserNotifications` 会把「能不能发通知」记在**代码签名标识**上。Tauri 的默认打包只做
linker-signed（adhoc），签名标识是 `didawork-<hash>`、`Info.plist` 也没封进签名，系统
会直接判定为不允许——表现就是「通知从来不弹，也没有任何提示」。

所以 `tauri.conf.json` 里显式配了 `bundle.macOS.signingIdentity: "-"`，让本地构建也做
正规的 ad-hoc 签名（`codesign -dvv` 的 `Identifier` 应等于 `com.bugu.didawork`）。
正式分发时用 `APPLE_SIGNING_IDENTITY` 环境变量覆盖成自己的 Developer ID 即可。

#### 开发模式的差异

`npm run tauri dev` 跑的是 `target/debug/didawork` 裸二进制，没有 `.app`、没有 bundle id，
`UNUserNotificationCenter` 在这种情况下会直接抛异常。所以那时自动退回
`mac-notification-sys`（借 Terminal 的身份），启动日志会打
`[notify] 开发模式：没有 bundle id，通知走降级通道（前台可能不弹）`。
**要看通知的真实行为，请用打包后的 `DiDaWork.app`。**

铃声到点（设置里的「铃声最长时间」）会自动收尾：声音停、会话清空、主界面横幅消失、
托盘菜单里的「关闭闹钟」变灰。之后再点旧通知的按钮也不会误停后来那一次铃声
（`ringer::dismiss_session` 用 `startedMs` 做过期校验）。

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

### 图标

应用图标（Dock、Finder、`tauri build` 产物）来自 `doc/icon/DiDaWork-icon.png`，
由 `npx tauri icon` 生成全套 `src-tauri/icons/*`。**菜单栏 / 托盘的常驻图标单独用
深色版** `doc/icon/DiDaWork-icon-dark.png`，在 `service/tray.rs` 里编译期嵌入
（`include_bytes!`），Tauri 会把它等比压到菜单栏的 18pt 高。按平台分两份：

| 平台 | 文件 | 说明 |
| --- | --- | --- |
| macOS | `icons/tray-mac.png` | 去掉白底、只剩黑形 + alpha，配合 `icon_as_template(true)` 交给系统着色：浅色菜单栏画黑、深色菜单栏画白，跟系统图标同一套规则 |
| Windows / Linux | `icons/tray.png` | 带白底圆角的 88px 版。这两个平台没有 template 概念，纯黑图形在深色任务栏 / 面板上会糊成一片 |

生成方式（Pillow 可用 `--proxy http://127.0.0.1:7890` 加速安装）：

```python
# 亮度 → 不透明度：白底全透明、图形全不透明，抗锯齿边缘自然过渡
alpha = im.convert("L").point(lambda v: max(0, min(255, (249 - v) * 255 // 249)))
# 裁到图形本体，按 90% 占位重排，再缩到 88px
```

Windows 侧若改了深色版源图，重新 `sips -Z 88` 生成 `tray.png` 即可。

### 铃声

铃声就是**一个文件夹**，放在数据目录的 `ringtones/` 下：

```
ringtones/
├── emotional-piano/          ← 文件夹名 = 列表里显示的名字
│   ├── ...-clip.m4a          ← 有 m4a / mp3 才算一首铃声（取文件名排序的第一首）
│   └── ...-license.txt       ← 有 txt 就当作版权声明
├── love-story/
└── medical/
```

- **内置三首**：`emotional-piano` / `love-story` / `medical`（均为 CC0，版权文本随文件夹一起放），
  源文件在 `src-tauri/resources/ringtones/`，由 `tauri.conf.json` 的 `bundle.resources` 打进安装包
  （运行时落在 `<App>.app/Contents/Resources/resources/ringtones`）。
  首次启动复制到数据目录，之后不再管——用户删掉哪首，下次启动不会自己长回来（靠 `ringtones/.seeded` 标记）。
- **自定义铃声**：闹钟编辑里铃声列表底部的「添加自定义铃声」→ 音乐名（必填）、音乐文件（必填，`.m4a` / `.mp3`）、
  版权声明（可选，`.txt`）。后端在 `ringtones/<音乐名>/` 下建目录并复制文件，跟内置的完全同构；
  所以也可以直接在访达里把歌拖进 `ringtones/` 新建一个文件夹，下次展开列表就能选到。
- **列表显示**：名字左对齐；文件夹里有 txt 时，右侧出现一个文档图标，点击用系统默认应用打开它。
- **播放**：不引入音频解码库，macOS `afplay`、Windows PowerShell + WPF `MediaPlayer`、
  Linux 依次尝试 `ffplay` / `mpv` / `gst-play-1.0` / `cvlc`。都是短命子进程，停铃即 kill；
  一首播完会接着循环，直到到达设置的铃声时长。
- **升级兼容**：老配置里存的是系统音效名（`glass` / `ping` …），启动时自动迁移到铃声库第一首。

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
ringtones/               铃声库：一个文件夹一首，首次启动时放入内置的三首
```

写入时机：设置/闹钟一改就落盘；进行中的时段每 5 分钟打一次检查点，
退出应用时再 flush 一次。所以**关窗口只是收进菜单栏，数据早已在磁盘上**，
即使强制结束进程，最多丢 5 分钟。设置页的「数据目录」一行会直接显示当前路径。
