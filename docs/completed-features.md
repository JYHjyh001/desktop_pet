# 已完成功能记录

本文档按模块记录已经完成并可维护的能力。具体需求、阶段拆分和验收细节保留在对应 `*-requirements.md`、`docs/releases/` 和专项说明文档中。

用户本机路径、AI API Key、Base URL、聊天配置、音乐文件、导入素材、平台凭据、Codex 会话内容和运行历史不得写入本文档。

## 维护规则

1. 新完成能力先归入下方既有模块；确实没有同类模块时再新增模块。
2. 单次小修不再重复堆叠流水条目，应更新模块摘要或对应需求文档状态。
3. 涉及前端界面的功能完成记录，必须确认默认主题和 `animal-island` 主题已检查或可继承现有样式。
4. 涉及用户本机数据的能力，只记录数据边界，不记录真实路径、账号、密钥、Cookie、token 或历史内容。

## 桌宠核心与快捷入口

- [x] 桌面宠物、快捷入口抽屉、宠物右键菜单、系统托盘和多窗口基础流程已完成。
- [x] 支持软件、文件夹、文件和网站快捷入口的添加、编辑、删除、搜索、分类、标签、常用和管理员启动。
- [x] 抽屉快捷卡片支持拖拽调整任意排列位置；筛选时仅重排可见卡片槽位，结果本机持久化并支持失败回滚，默认与 `animal-island` 主题均有拖动反馈。
- [x] 快捷入口启动时前端只传 `app_id`，后端从本机运行时数据读取路径并执行打开操作。
- [x] 支持开机自启、自动加入常用、窗口置顶、缩略/详细视图和运行时诊断信息。
- [x] Windows exe 高清图标提取和手动图标选择已完成；通用方法记录见 `docs/windows-executable-icon-extraction-method.md`，需求记录见 `docs/executable-icon-extraction-requirements.md`。

## 本机数据与隐私边界

- [x] 快捷入口、AI 配置、窗口设置、伴侣档案、宠物素材、图标、聊天记忆、音乐库和平台凭据都按模块保存到用户本机应用数据目录。
- [x] `.gitignore` 已覆盖 `.env*`、`config.json`、`apps.json`、记忆数据库、导入资源、打包产物、日志、证书和 token 类文件。
- [x] 发布和修改前隐私检查清单已建立，见 `docs/RELEASE_PRIVACY_CHECKLIST.md`。
- [x] 需求文档和 release notes 均使用通用描述，不写真实用户路径、密钥、Cookie、token、音乐文件或 Codex 内容。

## 发布安全、权限与工程基线

- [x] 更新页已移除 `cmd /C start`，仅允许当前项目的 GitHub HTTPS Release 地址，并覆盖命令元字符、异常主机和路径测试。
- [x] 本机歌词读取只接受后端音乐库登记的受支持音频关系；酷狗敏感认证字段不会再通过明文 HTTP 或不安全重定向发送。
- [x] Gemini API Key 已从 URL 查询参数迁到请求头；带 API Key 的非本机 AI 地址必须使用 HTTPS，敏感查询参数会被拒绝。
- [x] 普通窗口改读脱敏公开配置；原始 API Key、系统提示词、连接档案、Bridge Token、本机命令和路径只保留在设置链路。
- [x] Tauri capability 已按通用窗口、桌宠全局快捷键和本机文件选择拆分；CSP 已启用，本机媒体协议只运行时授权实际入库歌曲和已选择的受支持沉浸壁纸。
- [x] AI 聊天、故事和翻译界面已补外部发送提示，复用默认主题与 `animal-island` 主题的现有提示样式。
- [x] 窗口入口已改为 setup 阶段同步路由和按窗口动态导入，避免隐藏窗口先挂载抽屉及加载无关重型模块。
- [x] 已新增本机隐私门禁、前端/Rust 聚合检查和 Windows CI；本轮完整 Tauri Release 构建与安装包生成通过。
- [x] 高风险 Tauri command 已按 `drawer`、`pet`、`pet-chat`、`story`、`translator`、`music` 建立集中授权矩阵，并有允许、拒绝和只读例外测试。
- [x] 新安装首次启用划选翻译时会显示文本外发确认；旧配置通过兼容字段保持原快捷键，不擅自关闭已有功能。
- [x] `apps.json`、`config.json`、`storage.json`、音乐库及网易云/酷狗凭据已使用写入、同步和替换式原子落盘。
- [x] 音乐 Canvas/WebGL 在暂停、隐藏、降低动态或被覆盖时停止或降频；桌宠动作媒体使用最多两个候选的有界预加载缓存。
- [x] 前端已接入 Vitest 和 Tauri IPC mock；抽屉、聊天、故事、音乐和翻译已有 5 个测试文件、14 个回归用例，覆盖键盘导航、焦点约束/恢复、状态播报及默认/`animal-island` 代表状态；CI 增加 Windows 安装包构建任务。
- [x] 仓库已增加 Node 20、Rust stable 和统一行尾/二进制属性入口，降低本机与 CI 环境差异。
- [x] Rust Clippy 告警已清零，项目和 Windows CI 使用 `-D warnings` 阻止新增告警进入主线。
- [x] 抽屉入口/宠物素材/设置/确认框、聊天和音乐关键弹窗已补标题关联、初始焦点、Tab 边界、Escape 与焦点恢复；故事单选组支持方向键/Home/End，错误与状态变化可由读屏软件播报。
- [x] Three.js WebGL 运行时改为能力白名单与两级懒加载，最大块降至约 391.3 kB，完整 WebGL 懒加载体积减少约 25%，不再触发 Vite 500 kB 警告。
- [x] Release 图标构建已移除仓库外覆盖路径，使用 SHA-256 校验仓库内 ICO/ICNS 并接入 CI；第三方许可审计、Twemoji 应用内署名和安装包 notices 资源已建立。

## 主题与界面基础

- [x] 默认主题与 `animal-island` 主题基础适配已完成。
- [x] `animal-island` 主题已补齐主题 token、纸感卡片、胶囊控件、弹窗、音乐面板、宠物气泡、状态卡和错误提示样式，需求记录见 `docs/animal-island-theme-refresh-requirements.md`。
- [x] 新增窗口、弹窗、按钮、输入框、列表、空状态和错误提示的主题检查规则已写入项目注意事项。

## 宠物形象与操作绑定

- [x] 宠物形象库、内置默认宠物、导入宠物、编辑动画、预览和删除流程已完成。
- [x] 宠物动画状态模型已扩展到 `idle`、`hover`、`click`、`dragging`、`draggingLeft`、`draggingRight`、`waving`、`jumping`、`waiting`、`running`、`review` 和 `failed`，并保持旧版 4 状态宠物包兼容。
- [x] 拖动方向动画、状态回退规则、导入校验和默认素材回退已完成；后续 AI 素材转换计划见 `docs/ai-pet-material-generation-requirements.md`。
- [x] 宠物操作绑定支持单击、双击和右键分别绑定抽屉、Codex 完成优先、菜单、聊天、故事模式、音乐播放器或无操作，需求记录见 `docs/pet-action-bindings-requirements.md`。

## AI 伴侣、聊天与记忆

- [x] AI 接口配置支持 OpenAI 兼容、DeepSeek、Anthropic、Gemini、Ollama 和自定义服务；配置只保存到本机运行时数据。
- [x] 支持多伴侣档案、角色卡导入导出、人设、附加规则、模型覆盖、语音标识、关系状态和绑定形象。
- [x] 宠物聊天支持打字机回复、本地 Twemoji 表情、DeepSeek JSON Output 优先解析与普通请求降级。
- [x] 本机记忆系统已迁移到 SQLite + FTS5，支持按伴侣隔离的聊天记录、长期记忆、短期摘要、导入导出、手动维护和敏感信息跳过。
- [x] 好感度系统支持开关、AI/规则混合评分、关系阶段、心情、信任度、亲密度和关系变化日志。
- [x] 故事模式窗口、存档创建、推进、重命名和删除流程已接入现有伴侣和 AI 配置。

## Codex 状态聚合

- [x] Codex App Server 接入、运行时任务表、脱敏任务摘要、最近任务列表、未读完成/失败/待处理计数和确认入口已完成，需求记录见 `docs/codex-app-server-integration-requirements.md` 和 `docs/codex-multi-session-completion-notification-requirements.md`。
- [x] 宠物会根据聚合状态切换等待、处理中、检查、失败、完成等动画；全部任务完成后保留完成动画，直到用户悬浮或确认。
- [x] Codex 状态提醒改为悬浮触发和低打扰展示，不再主动弹出完成气泡或状态徽标。
- [x] 宠物气泡已拆成独立透明窗口，并支持多个气泡通道纵向展示、边界避让和双主题适配，需求记录见 `docs/pet-bubble-window-requirements.md`。
- [x] 宠物窗口尺寸改为按逻辑像素设置，在不同 DPI/缩放比例的显示器之间拖动时保持一致的视觉大小；隐藏宠物会先隐藏宠物窗口并阻止异步状态再次显示气泡。该窗口级修复同时适用于默认主题和 `animal-island` 主题。

## 翻译功能

- [x] 独立翻译窗口已完成，支持源/目标语言选择、直译/润色、交换语言、复制译文和双主题样式；翻译不写入聊天、记忆、好感度、故事、音乐或历史记录，需求记录见 `docs/translation-window-requirements.md`。
- [x] 划选翻译快捷键已完成，默认 `Ctrl+Alt+T`，支持用户自定义；只在运行态读取选中文本和译文，需求记录见 `docs/translation-selection-shortcut-requirements.md`。
- [x] 划选翻译气泡支持短译文、悬停展开完整译文、点击打开翻译窗口并填入本次运行态内容。
- [x] Windows PDF 剪贴板格式增强已完成，支持多种文本/富文本剪贴板格式和失败摘要；不做 OCR，不绕过复制限制，需求记录见 `docs/translation-selection-pdf-clipboard-enhancement-requirements.md`。

## 音乐播放器与本地音乐库

- [x] 音乐播放器基础、本机文件/文件夹导入、metadata 读取、内嵌封面、本机歌词、分类、标签、收藏、播放队列、播放历史和本机存储目录规则已完成，主需求见 `docs/music-feature-requirements.md`。
- [x] 本机音乐库持久层已迁到运行时 `music-library.json`，启动优先读取本机 JSON，并支持旧 `localStorage` 音乐数据迁移；索引位置跟随可自定义应用数据目录，需求记录见 `docs/music-local-library-persistence-requirements.md`。
- [x] 音乐库已补齐版本/结构校验、损坏备份保护、自定义目录下的平台凭据迁移、退出前 flush 和宠物聊天脱敏摘要；本机路径不会进入聊天意图上下文。
- [x] 音乐大列表已为歌曲 ID、首索引、有效 ID、队列位置、选歌状态和自建歌单解析建立响应式 Map/Set，逐行编号和成员判断不再重复扫描完整音乐库。
- [x] 普通音乐窗口布局已重设计为播放工作台和内容工作区，列表浏览区已扩展为宽版双栏；需求记录见 `docs/music-playback-layout-redesign-requirements.md` 和 `docs/music-list-browsing-redesign-requirements.md`。
- [x] 当前歌曲信息悬停详情、封面加载回退、歌单卡片布局和平台歌曲详情继续加载已完成。
- [x] 音乐迷你窗口贴边自动隐藏已完成，并适配默认主题和 `animal-island` 主题，需求记录见 `docs/music-mini-edge-autohide-requirements.md`。

## 在线音乐平台

- [x] 网易云音乐二维码登录、登录状态、本机凭据保存、歌单读取、歌曲搜索、在线歌词、YRC 逐字歌词和接口允许的临时在线播放已完成，需求记录见 `docs/music-netease-cloud-requirements.md`。
- [x] 酷狗音乐关键词搜索、二维码登录、个人歌单、推荐歌单、每日推荐、在线歌词和临时在线播放已完成，需求记录见 `docs/music-kugou-requirements.md` 和 `docs/music-kugou-recommendations-requirements.md`。
- [x] 在线平台歌单“加载更多”已改为按当前选择歌单的稳定 key 追加分页，避免详情接口返回的歌单标识变化导致重新加载第一页。
- [x] 酷狗播放链路已代理化，支持 Range 转发、临时播放链接刷新、播放链接预检、试听片段识别和脱敏诊断，需求记录见 `docs/music-kugou-playback-proxy-requirements.md`。
- [x] 在线音乐不可播放自动跳过、音质偏好、酷狗音质可用性预检查、播放中音质即时切换和平台会员信息显示已完成；相关需求见 `docs/music-online-*.md`、`docs/music-kugou-quality-compatibility-requirements.md` 和 `docs/music-platform-membership-display-requirements.md`。
- [x] 平台 Cookie、token、设备标识、播放 URL 和搜索结果只在本机运行态或本机配置中处理，不写入源码、本地音乐库或文档示例。

## 沉浸音乐与 WebGL 舞台

- [x] 沉浸式音乐模式基础已完成：播放与分析分离、本机节奏图、Web Audio 旁路频谱、Canvas 可视化、本机歌词沉浸显示、视觉模式、强度和降低动态控制，主需求见 `docs/music-immersive-mode-requirements.md`。
- [x] 沉浸模式播放列表已按“当前列表 / 我的歌单”拆分；当前列表随当前播放来源展示，我的歌单按本地、网易云和酷狗分组展示，点击歌单可原位展开歌曲并播放，需求记录见 `docs/music-local-library-persistence-requirements.md`。
- [x] 沉浸模式“我的歌单”已优化为统一歌单架：支持全部/本地/网易云/酷狗来源筛选、紧凑歌单卡、跨平台单一内联详情和详情头操作；网易云与酷狗刷新、读取详情和加载更多后都会恢复原展开歌单、选中歌单和滚动位置；当前歌曲已属于所选歌单时再次“播放歌单”会从当前歌曲和当前时间继续，不再跳到第一首；已完成默认主题与 `animal-island` 主题检查，需求记录见 `docs/music-immersive-playlist-rack-optimization-requirements.md`。
- [x] 沉浸模式独立主题、实时可视化样式、WebGL 星辰可视化、3D 可拖动舞台、自由镜头、只看舞台、顶部搜索和本机壁纸导入已完成，相关需求见 `docs/music-immersive-*.md`。
- [x] WebGL 星河和地面 DJ 两个舞台预设已完成；WebGL 继续负责星河、地形和音乐视觉，沉浸歌词由独立 DOM 文字层权威渲染，避免切歌后 CanvasTexture 当前句为空且必须重进沉浸模式的问题。
- [x] 地面 DJ 圆形声音地形、中心 Kick 冲击、波纹扩散、Shader 高度驱动、主题材质、柱体抬升高亮和歌词悬浮舞台已完成；参数说明见 `docs/music-dj-terrain-parameter-reference.md`。
- [x] 主歌词显示模型已重构，过滤歌名/文件名/间奏占位，加载、空态和错误态与真实歌词明确区分；进入沉浸模式或沉浸模式内切歌时会立即读取当前歌曲歌词；在线歌词使用独立超时边界且沉浸模式不再重复请求，慢速响应返回后无需重新进入沉浸模式；沉浸歌词已与 WebGL CanvasTexture 生命周期解耦，切歌与加载状态变化后由 DOM 文字层直接更新当前句，需求记录见 `docs/music-main-lyric-display-redesign-requirements.md` 和 `docs/music-immersive-lyrics-playback-stability-requirements.md`。

## 工程维护与文档

- [x] 版本发布记录已按版本保存在 `docs/releases/`。
- [x] 当前后续开发优先级和阶段路线记录在 `docs/next-development-plan.md`。
- [x] 本轮项目扫描、已清理代码和后续技术债记录在 `docs/project-code-audit-2026-07-07.md`。
