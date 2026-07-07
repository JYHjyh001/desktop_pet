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
- [x] 快捷入口启动时前端只传 `app_id`，后端从本机运行时数据读取路径并执行打开操作。
- [x] 支持开机自启、自动加入常用、窗口置顶、缩略/详细视图和运行时诊断信息。
- [x] Windows exe 高清图标提取和手动图标选择已完成；通用方法记录见 `docs/windows-executable-icon-extraction-method.md`，需求记录见 `docs/executable-icon-extraction-requirements.md`。

## 本机数据与隐私边界

- [x] 快捷入口、AI 配置、窗口设置、伴侣档案、宠物素材、图标、聊天记忆、音乐库和平台凭据都按模块保存到用户本机应用数据目录。
- [x] `.gitignore` 已覆盖 `.env*`、`config.json`、`apps.json`、记忆数据库、导入资源、打包产物、日志、证书和 token 类文件。
- [x] 发布和修改前隐私检查清单已建立，见 `docs/RELEASE_PRIVACY_CHECKLIST.md`。
- [x] 需求文档和 release notes 均使用通用描述，不写真实用户路径、密钥、Cookie、token、音乐文件或 Codex 内容。

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

## 翻译功能

- [x] 独立翻译窗口已完成，支持源/目标语言选择、直译/润色、交换语言、复制译文和双主题样式；翻译不写入聊天、记忆、好感度、故事、音乐或历史记录，需求记录见 `docs/translation-window-requirements.md`。
- [x] 划选翻译快捷键已完成，默认 `Ctrl+Alt+T`，支持用户自定义；只在运行态读取选中文本和译文，需求记录见 `docs/translation-selection-shortcut-requirements.md`。
- [x] 划选翻译气泡支持短译文、悬停展开完整译文、点击打开翻译窗口并填入本次运行态内容。
- [x] Windows PDF 剪贴板格式增强已完成，支持多种文本/富文本剪贴板格式和失败摘要；不做 OCR，不绕过复制限制，需求记录见 `docs/translation-selection-pdf-clipboard-enhancement-requirements.md`。

## 音乐播放器与本地音乐库

- [x] 音乐播放器基础、本机文件/文件夹导入、metadata 读取、内嵌封面、本机歌词、分类、标签、收藏、播放队列、播放历史和本机存储目录规则已完成，主需求见 `docs/music-feature-requirements.md`。
- [x] 普通音乐窗口布局已重设计为播放工作台和内容工作区，列表浏览区已扩展为宽版双栏；需求记录见 `docs/music-playback-layout-redesign-requirements.md` 和 `docs/music-list-browsing-redesign-requirements.md`。
- [x] 当前歌曲信息悬停详情、封面加载回退、歌单卡片布局和平台歌曲详情继续加载已完成。
- [x] 音乐迷你窗口贴边自动隐藏已完成，并适配默认主题和 `animal-island` 主题，需求记录见 `docs/music-mini-edge-autohide-requirements.md`。

## 在线音乐平台

- [x] 网易云音乐二维码登录、登录状态、本机凭据保存、歌单读取、歌曲搜索、在线歌词、YRC 逐字歌词和接口允许的临时在线播放已完成，需求记录见 `docs/music-netease-cloud-requirements.md`。
- [x] 酷狗音乐关键词搜索、二维码登录、个人歌单、推荐歌单、每日推荐、在线歌词和临时在线播放已完成，需求记录见 `docs/music-kugou-requirements.md` 和 `docs/music-kugou-recommendations-requirements.md`。
- [x] 酷狗播放链路已代理化，支持 Range 转发、临时播放链接刷新、播放链接预检、试听片段识别和脱敏诊断，需求记录见 `docs/music-kugou-playback-proxy-requirements.md`。
- [x] 在线音乐不可播放自动跳过、音质偏好、酷狗音质可用性预检查、播放中音质即时切换和平台会员信息显示已完成；相关需求见 `docs/music-online-*.md`、`docs/music-kugou-quality-compatibility-requirements.md` 和 `docs/music-platform-membership-display-requirements.md`。
- [x] 平台 Cookie、token、设备标识、播放 URL 和搜索结果只在本机运行态或本机配置中处理，不写入源码、本地音乐库或文档示例。

## 沉浸音乐与 WebGL 舞台

- [x] 沉浸式音乐模式基础已完成：播放与分析分离、本机节奏图、Web Audio 旁路频谱、Canvas 可视化、本机歌词沉浸显示、视觉模式、强度和降低动态控制，主需求见 `docs/music-immersive-mode-requirements.md`。
- [x] 沉浸模式独立主题、实时可视化样式、WebGL 星辰可视化、3D 可拖动舞台、自由镜头、只看舞台、顶部搜索和本机壁纸导入已完成，相关需求见 `docs/music-immersive-*.md`。
- [x] WebGL 星河和地面 DJ 两个舞台预设已完成，歌词已统一为 WebGL CanvasTexture 平面，DOM 歌词保留为 WebGL 不可用或无当前歌曲时的回退。
- [x] 地面 DJ 圆形声音地形、中心 Kick 冲击、波纹扩散、Shader 高度驱动、主题材质、柱体抬升高亮和歌词悬浮舞台已完成；参数说明见 `docs/music-dj-terrain-parameter-reference.md`。
- [x] 主歌词显示模型已重构，过滤歌名/文件名/间奏占位，加载、空态和错误态与真实歌词明确区分，需求记录见 `docs/music-main-lyric-display-redesign-requirements.md`。

## 工程维护与文档

- [x] 版本发布记录已按版本保存在 `docs/releases/`。
- [x] 当前后续开发优先级和阶段路线记录在 `docs/next-development-plan.md`。
- [x] 本轮项目扫描、已清理代码和后续技术债记录在 `docs/project-code-audit-2026-07-07.md`。
