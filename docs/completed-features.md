# 已完成功能记录

本文档记录项目中已经完成并可作为后续维护参考的功能。用户本机路径、配置、密钥、导入资源和运行历史不得写入本文档。

## 当前记录

- [x] 桌面宠物与快捷入口抽屉基础功能。
- [x] 软件、文件夹、文件和网站快捷入口管理。
- [x] 本机应用数据目录保存快捷入口、配置、宠物素材和图标资源。
- [x] 默认主题与 `animal-island` 主题基础适配。
- [x] 音乐播放功能的阶段性实现记录见 `docs/music-feature-requirements.md`。
- [x] 抽屉软件图标高清提取增强，支持 exe 自动提取和手动选择 exe、lnk、ico 图标来源。
- [x] Windows exe 图标提取通用方法文档，见 `docs/windows-executable-icon-extraction-method.md`。
- [x] 宠物 Codex 状态气泡独立窗口，支持宠物大小变化后的屏幕边界避让和主题适配，需求计划见 `docs/pet-bubble-window-requirements.md`。
- [x] Codex 状态提醒改为悬浮触发：任务状态变化只保留宠物动作，不再主动弹出状态气泡、状态徽标或完成徽标，避免完成提醒与悬浮气泡重叠。
- [x] Codex 全部任务完成后保持完成动画，直到用户将鼠标悬浮到宠物上；多任务完成时在宠物右下角显示轻量完成数量角标。
- [x] Codex 多会话/多任务状态聚合，运行时维护脱敏任务表、聚合摘要、未读完成/失败/待处理计数、设置页最近任务列表和确认入口，需求计划见 `docs/codex-multi-session-completion-notification-requirements.md`。
- [x] 宠物操作绑定可在设置中调整，支持单击、双击和右键分别绑定抽屉、Codex 完成优先、菜单、聊天、故事模式、音乐播放器或无操作，需求计划见 `docs/pet-action-bindings-requirements.md`。
- [x] `animal-island` 主题对齐 `animal-island-ui` 最新 main 做渐进式增强，补齐主题 token、纸感卡片、胶囊控件、弹窗、音乐面板和宠物气泡样式，需求计划见 `docs/animal-island-theme-refresh-requirements.md`。
- [x] 宠物动画状态模型扩展到多状态形象包，支持 `idle`、`hover`、`click`、`dragging`、左右拖动、打招呼、跳跃、等待、处理中、检查和失败状态的导入、编辑、预览与旧宠物包回退，后续计划见 `docs/next-development-plan.md`。
- [x] 沉浸式音乐模式第一阶段：在现有音乐窗口中加入播放与分析分离的本机节奏图、Web Audio 旁路实时频谱、Canvas 沉浸可视化、本机同名歌词沉浸显示、视觉模式切换、强度和降低动态控制，并补齐默认主题与 `animal-island` 主题样式；后续计划见 `docs/music-immersive-mode-requirements.md`。
- [x] 网易云音乐接入第三阶段：音乐窗口新增网易云页签，支持二维码登录、扫码状态轮询、本机凭据保存、登录状态读取、本机退出清除、当前账号歌单列表读取、歌单歌曲摘要展示、在线歌词读取和接口允许的临时在线播放；Cookie 和本机路径不返回前端，播放 URL 只用于当前运行态播放，不写入本地音乐库；后续计划见 `docs/music-netease-cloud-requirements.md`。
- [x] 网易云音乐 YRC 逐字歌词：后端分离返回 LRC/YRC/K 歌词字段，前端优先使用 YRC 片段时间驱动沉浸歌词逐字进度，缺少 YRC 时自动回退普通歌词同步；后续计划见 `docs/music-netease-cloud-requirements.md`。
