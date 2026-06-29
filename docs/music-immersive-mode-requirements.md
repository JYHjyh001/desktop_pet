# 沉浸式音乐模式需求计划

本文档记录“沉浸式音乐模式”的调研结论、产品方案、技术路线和分阶段实现计划。该模式基于现有本地音乐播放器扩展，不改变“歌曲路径、播放列表、歌曲文件和 metadata 只保存在用户本机运行时数据中”的原则。

## 目标

- 在现有音乐播放器中增加“沉浸”入口，让用户可以从普通播放器切换到大画布音乐可视化体验。
- 根据本地音频在本机生成节奏能量图或旁路分析低频、中频、高频能量，驱动画面、控件氛围和桌宠动作状态。
- 统一承接后续“跳舞模式、专注模式、睡眠模式、剧情音乐模式、宠物动画联动”等音乐待办功能。
- 默认主题和 `animal-island` 主题都必须有可用且风格一致的界面表现。
- 所有音频分析都在本机完成，不上传原始音频、音频指纹、歌曲路径、播放历史或识别密钥。

## GitHub 可行方案调研

| 方案 | 仓库 | 授权与定位 | 适合程度 | 结论 |
| --- | --- | --- | --- | --- |
| Butterchurn | [jberg/butterchurn](https://github.com/jberg/butterchurn) | MIT；JavaScript/WebGL 版 MilkDrop 可视化器 | 很适合全屏沉浸主视觉 | 推荐作为第二阶段主引擎，配合 [butterchurn-presets](https://github.com/jberg/butterchurn-presets) 使用预设。 |
| WaveSurfer.js | [katspaugh/wavesurfer.js](https://github.com/katspaugh/wavesurfer.js) | BSD-3-Clause；音频波形播放器和插件体系 | 适合波形、时间线、歌词/段落扩展 | 不建议替换当前播放器核心，适合作为后续波形/歌词时间线层。 |
| Vue Audio Visual | [staskobzar/vue-audio-visual](https://github.com/staskobzar/vue-audio-visual) | MIT；Vue 音频可视化组件 | 适合快速做频谱、圆形、波形等轻量视觉 | 推荐作为 MVP 参考或直接集成候选，能降低 Vue 项目接入成本。 |
| audioMotion-analyzer | [hvianna/audioMotion-analyzer](https://github.com/hvianna/audioMotion-analyzer) | AGPL-3.0-or-later；高质量实时频谱分析器 | 视觉质量高，但授权风险高 | 不建议直接并入当前项目；可参考交互和参数设计。 |
| PowerAudio | [7PH/poweraudio](https://github.com/7PH/poweraudio) | README 和仓库侧栏授权信息需复核；偏 Windows 系统音频捕获 + PixiJS | 适合未来研究外部播放器/system audio | 不进入近期实现，只作为第三方音乐软件控制阶段的架构参考。 |

## 推荐技术路线

### 总体选择

采用“播放与分析分离 + 两层可视化”架构：

1. 本机节奏图层：保留现有 `<audio>` 只负责播放，后台读取当前歌曲 URL 并通过 `decodeAudioData` 在内存里生成 bass、mid、treble、volume、beat 帧，渲染时用 `audio.currentTime` 对齐节奏图。
2. 旁路实时层：仅在 WebView 支持 `captureStream` 时补充实时频谱，不使用会接管播放输出的 `createMediaElementSource` 回退。
3. 沉浸主视觉层：后续以 Butterchurn + presets 作为 WebGL 主视觉，按设备性能和用户设置启用。

不推荐在第一阶段直接替换播放器核心。当前 `MusicWindow.vue` 已经承载导入、播放、队列、歌单、AI 控制和迷你播放器，沉浸模式应复用同一个音频元素和播放状态，避免出现双播放器、双进度和重复播放。

### 音频分析原则

- 优先使用“播放与分析分离”：不把 `<audio>` 连接到 `AudioContext.destination`，避免 WebView2 中出现可视化有效但播放无声的问题。
- 旁路实时分析只使用 `captureStream` 创建 `MediaStreamAudioSourceNode`；如果当前 WebView 不支持，则回退到节奏图或安全待机视觉。
- `AudioContext` 只在用户触发播放、进入沉浸模式或需要本机解码节奏图时创建，符合浏览器自动播放限制。
- `AnalyserNode` 输出数据只保存在内存中，用于当前帧渲染和桌宠状态，不写入源码、文档或提交。
- 分析结果只保留抽象能量值，例如 `bassEnergy`、`midEnergy`、`trebleEnergy`、`beatPulse`、`energyLevel`，不保存音频指纹。
- 歌词只支持本机文件读取，优先查找当前歌曲同目录同名 `.lrc` 或 `.txt`，不接入外部歌词搜索、音频指纹识别或上传服务。

## 产品设计

### 入口

- 在音乐窗口顶部操作区增加“沉浸”按钮，和现有“迷你”“设置”并列。
- AI 音乐动作协议后续可新增 `start_immersive_mode`，用户说“进入沉浸音乐模式”“让桌宠跟着音乐跳舞”时可以触发。
- 场景歌单和 AI 推荐卡片可增加“沉浸播放”入口，先播放匹配歌曲，再进入沉浸界面。

### 界面布局

沉浸模式仍在 `music` 窗口内实现第一版，进入时直接切换当前音乐窗口为全屏；退出沉浸、切换迷你播放器或隐藏播放器时恢复非全屏。后续如需多屏选择或独立桌面氛围层，再评估独立 `music-immersive` 窗口。

界面分区：

- 背景层：全窗口 canvas/WebGL 可视化。
- 顶部层：当前歌曲名、歌手、播放状态、退出沉浸、最小化、设置。
- 歌词层：居中显示当前歌词句，上一句和下一句以半透明纵深排布；没有本机歌词文件时显示低打扰的歌曲信息，不联网检索歌词。
- 底部层：播放/暂停、上一首、下一首、进度条、音量、循环/随机。
- 侧边层：视觉模式、预设、强度、降低动态效果、桌宠联动开关。
- 空状态：未播放时展示可点击的本地音乐入口，不展示示例歌曲路径。
- 错误状态：WebGL 不可用或音频解码失败时回退到轻量频谱层，并显示简短错误提示。

### 视觉模式

1. 韵律空间：默认沉浸模式，频谱线、光带、粒子和节拍脉冲跟随音乐变化。
2. 跳舞模式：高能量歌曲或用户手动开启时使用更强的节拍脉冲，并驱动宠物进入 `music_dance`。
3. 专注模式：低动态、低亮度、减少粒子，显示专注计时和简化控制。
4. 睡眠模式：低亮度、慢速呼吸动效、音量渐弱、定时停止。
5. 剧情音乐模式：故事模式按场景传入 `sceneMood`，映射到温柔、悬疑、热血、伤感等视觉氛围。

### 主题适配

默认主题：

- 以深色背景、柔和高对比频谱、半透明控制栏为主。
- 按当前播放状态改变强调色，但控制文本必须保持可读。
- 所有按钮、滑块、错误提示和空状态都要能继承现有音乐窗口样式。

`animal-island` 主题：

- 使用现有 `animal-island` token，控制栏保持纸感和岛屿风格。
- 可视化颜色更柔和，避免整屏高饱和闪烁。
- 侧边设置、空状态、错误提示、按钮、输入项必须提供主题样式或确认可继承现有主题样式。

可访问性：

- 提供“降低动态效果”开关，关闭强闪烁、强缩放和快速旋转。
- 视觉强度默认不超过中等，用户可手动增强。
- 当系统或应用检测到低性能时，自动降级到轻量频谱。

## 状态与数据设计

新增本地设置建议：

```ts
interface MusicVisualizerSettings {
  immersiveEnabled: boolean
  engine: 'simple' | 'butterchurn'
  visualMode: 'rhythm' | 'dance' | 'focus' | 'sleep' | 'story'
  presetId: string
  intensity: number
  reducedMotion: boolean
  petReactionEnabled: boolean
  autoDanceEnabled: boolean
  showWaveform: boolean
}
```

这些设置可以先跟随现有音乐设置存入本机运行时存储。不得把用户歌曲路径、歌曲名列表、播放历史或导入资源写入源码、示例文档或仓库。

运行态分析数据建议：

```ts
interface MusicEnergyFrame {
  bass: number
  mid: number
  treble: number
  volume: number
  beat: number
  mood: 'calm' | 'warm' | 'energetic' | 'sleep' | 'focus'
}
```

该数据仅用于实时渲染和桌宠动作联动，不落盘。

## 与桌宠联动

沉浸模式输出抽象状态给宠物窗口：

- `music_idle`：未播放或无可分析音频。
- `music_playing`：播放中但节奏不强。
- `music_dance`：低频和节拍强，或用户开启跳舞模式。
- `music_focus`：专注模式。
- `music_sleep`：睡眠模式。
- `music_sad`：伤感/低能量陪伴。
- `music_happy`：明亮/中高能量陪伴。

联动强度建议分三档：

- 低：宠物轻微点头或呼吸，不影响工作。
- 中：尾巴、耳朵、身体轻微律动。
- 高：跳舞、音符粒子、节拍光圈，仅在用户主动开启或跳舞模式中使用。

## 推荐开发阶段

### 第一阶段：轻量沉浸 MVP

- [x] 在 `MusicWindow.vue` 增加 `immersiveMode` 状态和“沉浸”按钮。
- [x] 新增 `useMusicAudioAnalyzer`，从现有 `<audio>` 元素读取实时频谱。
- [x] 新增 `useMusicBeatMapAnalyzer`，参考 Mineradio 的思路实现播放与分析分离：后台本机解码当前歌曲、生成抽象节奏图，并按播放进度驱动画布。
- [x] 沉浸模式使用专用视觉时钟读取 `audio.currentTime`，并对节奏图前后帧做插值，避免 `timeupdate` 低频更新导致画面卡顿。
- [x] 新增本机同名歌词读取和沉浸歌词层，当前句居中高亮、上下句虚化显示，歌词内容只保留在运行时内存中。
- [x] 新增 `MusicVisualizerCanvas`，先实现 canvas 频谱、光带和节拍脉冲。
- [x] 复用当前播放、暂停、进度、音量、切歌和队列逻辑。
- [x] 补齐默认主题和 `animal-island` 主题样式。
- [x] 完成未播放、无歌曲、音频错误、分析器初始化失败和降低动态效果的基础状态。
- [ ] WebGL 不可用状态随第二阶段 Butterchurn 主视觉一起处理。

### 第二阶段：Butterchurn 主视觉

- 引入 `butterchurn` 和 `butterchurn-presets`。
- 采用动态加载，只有进入沉浸模式且 WebGL 可用时加载。
- 增加预设选择、随机预设、强度控制和性能降级。
- 当 FPS 或渲染耗时异常时回退到轻量 canvas。

### 第三阶段：宠物动画联动

- 定义音乐能量到宠物状态的映射。
- 通过 Tauri event 或现有窗口事件把抽象状态发送给宠物窗口。
- 为默认宠物和自定义宠物包提供缺省回退动作，缺少音乐动画时回退到 `idle`、`waving`、`jumping`。

### 第四阶段：专注、睡眠、剧情音乐

- 专注模式加入计时、低动态、低打扰策略。
- 睡眠模式加入音量渐弱、定时停止、宠物睡眠动作。
- 剧情音乐模式由故事窗口传入场景情绪，沉浸模式只接收安全的场景标签，不接收敏感剧情原文。

### 第五阶段：波形和外部音频研究

- 按需评估 WaveSurfer.js，用于波形、歌词时间线、段落标记。
- 第三方音乐软件或系统音频捕获另立调研，不和本地播放器 MVP 混在一起。
- PowerAudio 类方案涉及 Windows 系统音频捕获和授权复核，只能作为后续研究项。

## 涉及文件建议

- `src/windows/MusicWindow.vue`：入口、模式状态、布局切换、播放器状态复用。
- `src/components/MusicVisualizerCanvas.vue`：轻量 canvas 可视化组件。
- `src/composables/useMusicAudioAnalyzer.ts`：AudioContext、AnalyserNode、频谱采样和生命周期。
- `src/composables/useMusicBeatMapAnalyzer.ts`：本机音频解码、抽象节奏图生成和播放进度对齐。
- `src/composables/useMusicLyrics.ts`：同名 `.lrc`/`.txt` 读取、解析和播放进度匹配。
- `src/types/app.ts`：音乐可视化设置和音乐状态类型。
- `src/styles.css`：默认主题和 `animal-island` 主题样式。
- `src-tauri/src/windowing.rs`：如需要沉浸窗口尺寸和位置，扩展音乐窗口布局逻辑。
- `docs/music-feature-requirements.md`：后续实现时同步勾选阶段进度。

## 验收标准

- 可以从普通音乐窗口进入和退出沉浸模式，播放状态不丢失。
- 当前歌曲播放时可视化实时响应节奏；暂停后画面降为静态或慢速待机。
- 未播放、无歌曲、音频错误、WebGL 不可用都有明确状态。
- 默认主题和 `animal-island` 主题下按钮、滑块、空状态、错误提示、侧边设置均可读且不重叠。
- 不上传音频、路径、metadata、播放历史或音频指纹。
- 不把用户本机音乐路径、导入资源、配置文件或缓存写入仓库。
- 执行 `npm run build` 通过；需要完整桌宠 UI 验证时按项目规则执行 `npm run tauri:build` 后启动 release exe。

## 风险与处理

- WebGL 性能不稳定：默认先用轻量 canvas，Butterchurn 动态加载并支持回退。
- 授权风险：不直接使用 AGPL 的 audioMotion-analyzer；PowerAudio 进入后续复核，不纳入近期依赖。
- 双重播放风险：不新建第二个播放器，复用当前 `<audio>` 元素和播放状态。
- 视觉刺激过强：提供降低动态效果和强度控制，睡眠/专注默认低动态。
- 主题割裂：每次新增 UI 都同时检查默认主题和 `animal-island` 主题。
