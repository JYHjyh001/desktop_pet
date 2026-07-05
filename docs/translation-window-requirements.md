# 翻译窗口功能需求计划

状态：已实现。

本文档规划 PetDrawer 第一阶段翻译功能。翻译是独立窗口和独立功能，不作为宠物聊天、故事模式、音乐窗口或抽屉设置的附属面板实现。第一阶段采用独立窗口方式，不做系统级划词翻译、OCR 或全局快捷键。用户输入文本、翻译结果、语言偏好和 AI 配置都按隐私边界处理，不把用户文本、API Key、Base URL、模型配置或翻译历史写入源码、文档示例或仓库数据文件。

## 搜索结论

### 当前项目可复用能力

1. 现有 Tauri 窗口采用固定 label 注册，前端 `App.vue` 根据 `getCurrentWindow().label` 渲染对应 Vue 窗口组件。
2. `windowing.rs` 已有 `show_pet_chat`、`show_story`、`show_music_player` 的窗口定位、显示、聚焦和开启动画模式，翻译窗口可以沿用同一模式。
3. `ai_chat.rs` 已支持 OpenAI 兼容、DeepSeek、Anthropic、Gemini 和 Ollama，并且 AI 配置只保存在用户本机 `config.json`。
4. `ai_chat.rs` 已有 `send_story_text_request` 这种不写入宠物记忆、不走伴侣好感度的纯文本请求路径，翻译功能适合复用或抽出同类函数。
5. 当前项目没有引入剪贴板插件或全局快捷键插件，第一阶段不依赖这些能力，避免扩大权限。

### 外部方案对比

1. Azure AI Translator、Google Cloud Translation、DeepL API 都是专用翻译 API，适合后续做可选翻译引擎，但会新增服务密钥、计费和供应商配置。
2. OpenAI 兼容模型和本地 Ollama 可直接复用现有 AI 配置，第一阶段改动最小；缺点是译文稳定性依赖用户所选模型。
3. 因为 PetDrawer 已经有 AI 接口设置页，第一阶段建议默认使用“现有 AI 接口翻译”，不新增翻译服务密钥。

参考来源：

- Microsoft Azure AI Translator 文档：https://learn.microsoft.com/azure/ai-services/translator/
- Google Cloud Translation 文档：https://cloud.google.com/translate/docs
- DeepL API 文档：https://developers.deepl.com/docs/api-reference/translate
- OpenAI API 文档：https://platform.openai.com/docs

## 目标

1. 新增独立翻译窗口和独立翻译业务命令，用户可以从宠物右键菜单和抽屉入口打开。
2. 支持输入待翻译文本、选择源语言、目标语言并生成译文。
3. 默认支持自动识别源语言，目标语言默认中文。
4. 支持复制译文、清空文本、交换源/目标语言、保留换行和列表结构。
5. 复用现有 AI 接口配置，不新增必填密钥。
6. 翻译请求不进入宠物聊天记录、不写入宠物记忆、不影响好感度系统。
7. 默认主题和 `animal-island` 主题都要有完整窗口、按钮、输入框、下拉框、空状态、加载态和错误提示样式。

## 独立功能边界

1. 翻译窗口使用独立 Tauri label：`translator`。
2. 翻译前端使用独立组件：`src/windows/TranslatorWindow.vue`。
3. 翻译后端使用独立命令：`translate_text`，不复用 `send_pet_chat_message` 作为业务入口。
4. 翻译提示词使用固定翻译助手系统提示词，不继承当前伴侣人格、故事设定、音乐上下文或聊天后处理指令。
5. 翻译状态只存在于翻译窗口运行态，不同步到宠物聊天窗口、故事窗口、音乐窗口或抽屉列表。
6. 宠物右键菜单和抽屉只作为打开入口，不承载翻译表单、翻译结果或翻译历史。
7. 后续增加翻译历史、剪贴板或专用翻译 API 时，也应保持为翻译模块自己的配置和数据边界。

## 非目标

1. 第一阶段不做系统级划词翻译。
2. 第一阶段不注册全局快捷键。
3. 第一阶段不读取系统剪贴板内容。
4. 第一阶段不做截图 OCR 或图片翻译。
5. 第一阶段不保存翻译历史。
6. 第一阶段不新增 Azure、Google、DeepL 等独立翻译服务配置。
7. 不把用户翻译文本、译文、使用历史或外部服务响应写入仓库、示例文档或日志。
8. 不把翻译功能做成宠物聊天的快捷提问、故事模式的工具栏、音乐窗口的子面板或抽屉设置里的表单。

## 窗口方案

### Tauri 窗口

新增窗口 label：

- `translator`

建议窗口配置：

- 标题：`PetDrawer Translator`
- 尺寸：`720 x 520`
- `decorations: false`
- `transparent: true`
- `resizable: false`
- `visible: false`
- `alwaysOnTop: true`
- `skipTaskbar: true`

### 打开方式

1. 宠物右键菜单新增“翻译”按钮，调用 `show_translator`。
2. 抽屉窗口顶部或设置旁新增“翻译”入口，调用 `show_translator`。
3. 后续如果要接入宠物操作绑定，可把“打开翻译窗口”加入动作枚举，但不作为第一阶段必做项。

### 定位规则

沿用 `position_story` / `position_music_player`：

1. 优先显示在宠物右侧。
2. 右侧空间不足时显示在宠物左侧。
3. 垂直方向限制在当前显示器可视区域内。
4. 使用 `show_window_with_open_animation(..., "panel", true)`。

## 前端交互

新增组件：

- `src/windows/TranslatorWindow.vue`

推荐布局：

1. 顶部标题栏：标题、当前引擎摘要、关闭按钮。
2. 语言栏：源语言下拉、交换按钮、目标语言下拉。
3. 输入区：多行文本框，显示字数，支持清空。
4. 操作栏：翻译按钮、复制译文按钮、清空按钮。
5. 输出区：只读译文，多行保留格式。
6. 状态区：AI 未启用、模型缺失、翻译中、失败、空输入等状态提示。

语言第一阶段建议内置：

- 自动识别
- 中文
- 英文
- 日文
- 韩文
- 法文
- 德文
- 西班牙文
- 俄文
- 意大利文
- 葡萄牙文

## 后端命令

新增命令建议：

- `show_translator`
- `hide_translator`
- `translate_text`

`translate_text` 请求结构：

```text
sourceLanguage: string
targetLanguage: string
text: string
mode: "plain" | "polish"
```

`translate_text` 返回结构：

```text
translatedText: string
detectedLanguage?: string
provider: string
model: string
```

第一阶段 `mode` 默认只使用 `plain`，保留字段方便后续增加“润色翻译”。

## AI 提示词策略

系统提示词固定为翻译助手，不继承宠物人格：

```text
你是 PetDrawer 的翻译助手。只完成翻译任务，不闲聊，不添加解释。保持原文段落、列表、换行、标点和代码块结构。不要保存或复述隐私说明。无法确定源语言时根据文本自动判断。
```

用户消息包含：

```text
源语言：自动识别
目标语言：中文
翻译模式：plain
文本：
...
```

参数建议：

1. `temperature = 0.1`
2. `max_tokens` 取现有配置并限制到合理范围，例如 `512..4096`
3. 输入文本第一阶段限制 6000 字符，超过时提示用户拆分，避免长文本导致请求失败或费用不可控。

## 隐私与数据边界

1. 翻译文本只在当前窗口运行态保存。
2. 关闭窗口后不保留输入和译文，除非后续明确增加本机历史功能。
3. 翻译请求不写入宠物消息、宠物记忆、故事存档、音乐历史或应用源码。
4. 如果用户使用在线 AI 服务，文本会发送到用户配置的模型服务；界面需要明确提示“将通过当前 AI 接口翻译”。
5. 如果用户选择本地 Ollama，则请求只走用户配置的本机服务地址。
6. 错误提示不得输出 API Key、完整 Base URL 查询参数或完整请求体。

## 主题要求

默认主题：

1. 使用现有面板、标题栏、二级按钮、输入框和状态提示风格。
2. 输入区和输出区需要明确分区，文本较多时内部滚动，不撑破窗口。
3. 加载态按钮禁用并显示“翻译中”。

`animal-island` 主题：

1. 翻译窗口根节点必须带 `theme-animal-island`。
2. 标题栏、语言下拉、交换按钮、输入框、输出框、复制按钮、错误提示、空状态都要有对应纸感/暖色样式，或确认继承现有主题选择器。
3. 小窗口下按钮文字不能溢出，输入输出区高度稳定。

## 实现步骤

1. 新增 `docs/translation-window-requirements.md` 并加入未完成功能记录。
2. 在 `tauri.conf.json` 增加 `translator` 窗口配置。
3. 在 `src-tauri/capabilities/default.json` 的 `windows` 加入 `translator`。
4. 在 `windowing.rs` 增加 `show_translator`、`hide_translator` 和 `position_translator`。
5. 在 `commands.rs` 暴露窗口命令和独立 `translate_text` 命令。
6. 在 `lib.rs` 注册命令。
7. 在 `ai_chat.rs` 新增纯翻译请求函数，复用现有 provider 请求构建和响应解析，但不走宠物聊天记忆、好感度、音乐意图或故事存档链路。
8. 在 `App.vue` 加入 `TranslatorWindow` 路由。
9. 新增 `TranslatorWindow.vue`，实现输入、语言选择、交换、翻译、复制和错误状态。
10. 在 `PetMenuWindow.vue` 和 `DrawerWindow.vue` 增加入口。
11. 在 `styles.css` 补充默认主题和 `animal-island` 主题样式。
12. 验证默认主题和 `animal-island` 主题下窗口、按钮、输入框、空状态、加载态、错误提示不溢出。

## 验收标准

1. 从宠物右键菜单可以打开翻译窗口。
2. 从抽屉可以打开翻译窗口。
3. AI 未启用时，窗口显示引导提示，不发起请求。
4. 输入为空时，翻译按钮禁用或显示空输入提示。
5. 输入中文、英文、日文等文本后可以翻译为目标语言。
6. 翻译结果保留段落和列表结构。
7. 复制译文可用；如果浏览器剪贴板不可用，显示可理解错误。
8. 翻译请求不写入宠物聊天记录、不写入记忆、不修改好感度。
9. 翻译窗口关闭和重新打开时不依赖聊天、故事或音乐窗口状态。
10. 默认主题和 `animal-island` 主题都完成界面检查。
11. `npm run build` 通过。

## 后续增强

1. 本机翻译历史，默认关闭，只保存到本机运行时数据目录。
2. 支持 DeepL、Azure、Google 等专用翻译引擎作为可选配置。
3. 支持读取剪贴板并一键翻译，需要引入 Tauri 剪贴板插件和权限。
4. 支持全局快捷键打开翻译窗口，需要引入全局快捷键插件和用户可配置快捷键。
5. 支持截图 OCR 翻译，需要额外的本机截图、OCR 和隐私提示设计。
6. 支持选中文本快速翻译，但需要评估 Windows 权限、焦点恢复和误触成本。
