# Codex App Server 状态接入需求计划

## 背景

用户希望 AI 宠物直接接入 Codex App Server，获取 Codex 工作状态，并在 Codex 工作完成、等待用户输入、失败或正在处理时提醒用户。

## 目标

1. 桌宠应用默认通过 `codex app-server proxy` 连接当前正在运行的 Codex App 控制通道。
2. 用户在 Codex 中开始任务时，桌宠可以接收该控制通道上的线程和轮次状态事件。
3. 桌宠保留独立测试模式，用于启动本机 WebSocket App Server 并验证宠物状态链路。
4. Windows Codex Desktop 当前没有公开本机 control socket 时，桌宠提供“日志监听模式”，监听本机 Codex 会话 JSONL 的新增状态事件，实现完成提醒。
5. 宠物根据 Codex 事件切换动画状态：
   - 空闲：`idle`
   - 正在工作：`running`
   - 等待用户输入或审批：`waiting`
   - 审查/总结：`review`
   - 完成：`jumping` 或 `waving`
   - 失败：`failed`
6. 宠物窗口显示简短状态气泡，例如“Codex 正在工作”“Codex 需要你处理”“Codex 工作完成”。
7. 所有 Codex 状态数据仅在本机运行时处理，不写入源码、示例或仓库。

## 非目标

1. 不上传 Codex 消息、项目路径、终端输出、工作区内容或用户隐私数据到第三方服务。
2. 不保存完整 Codex 对话历史。
3. 不替代 Codex 官方客户端，只提供桌宠侧状态提醒和最小测试入口。
4. 不主动读取或展示 Codex 对话正文；只保留线程 ID、轮次 ID、状态、事件名和错误摘要。
5. 不把 Codex 会话日志的 prompt、assistant 正文、工具输出、工作区路径或本机绝对路径复制到前端状态、源码、文档示例或持久化配置。

## 方案边界

Codex App Server 是自定义客户端与 Codex 会话通信的协议入口。Windows Codex Desktop 没有公开本机 control socket 时，补充使用会话日志监听。当前采用三种模式：

1. 默认 `proxy` 模式：Tauri 后端启动 `codex app-server proxy`，通过标准输入/输出连接正在运行的 Codex App 控制 socket。
2. `proxy` 模式初始化后调用 `thread/loaded/list`，对已加载线程调用 `thread/resume`，之后监听 `thread/status/changed`、`turn/started`、`turn/completed` 和 `item/*` 事件。
3. `managed` 独立测试模式：Tauri 后端启动 `codex app-server --listen ws://127.0.0.1:<port>`，再作为客户端连接该 WebSocket。
4. `sessionLog` 日志监听模式：Tauri 后端监听 `CODEX_HOME/sessions` 或 `~/.codex/sessions` 下新增/追加的 `.jsonl` 行，只解析事件类型、阶段、角色和状态，不读取或展示 prompt 与 assistant 正文。
5. `proxy` 模式支持填写自定义 `--sock` 路径；留空时使用 Codex 默认 control socket。
6. `managed` 模式允许从设置页提交短测试任务，也显示 `codex --remote ws://127.0.0.1:<port>` 命令，用户可用 Codex CLI 连接到桌宠启动的 App Server 后开始任务。
7. 后端解析 App Server 事件或会话日志事件，并通过 Tauri 事件广播到宠物窗口和抽屉窗口。

如果本机未安装 Codex CLI，或 `codex app-server proxy` 无法连接当前 Codex 控制 socket，应展示可理解的错误提示，不影响桌宠其他功能。

## 功能清单

### 后端

1. 新增 Codex 集成配置：
   - 是否启用
   - 连接模式：`proxy` 监听当前 Codex control socket，`managed` 独立测试，`sessionLog` 监听 Codex 会话日志
   - Codex 命令路径，默认 `codex`
   - Control socket 路径，仅 `proxy` 模式使用，默认留空
   - WebSocket 端口，仅 `managed` 模式使用，默认本机高位端口
   - 完成提醒是否启用
2. 新增运行时状态：
   - `disconnected`
   - `starting`
   - `connected`
   - `running`
   - `waiting`
   - `review`
   - `completed`
   - `failed`
3. 日志监听模式新增运行要求：
   - 启动时记录已有 `.jsonl` 文件读取偏移，避免旧任务重复提醒
   - 新文件从头读取，捕获新任务的 `task_started`、`final_answer`、`task_complete` 等事件
   - 同一任务完成事件只提醒一次，避免 `final_answer` 和 `task_complete` 重复触发
4. 新增 Tauri 命令：
   - 获取 Codex 状态
   - 启动/连接 Codex App Server 或当前 Codex proxy
   - 停止 Codex App Server
   - 启动一个 Codex 任务
5. 新增 Tauri 事件：
   - `codex-status-updated`
   - 复用 `pet-animation-state`

### 前端

1. 设置页新增“Codex 状态提醒”区域。
2. 提供连接模式选择、连接、断开、刷新入口。
3. `proxy` 模式提供自定义 control socket 路径输入，并提示用户直接在 Codex 中开始任务。
4. `sessionLog` 模式说明适用于 Windows Codex Desktop 完成提醒，不需要 `sock` 或 `--remote`。
5. `managed` 模式提供测试任务入口和 `codex --remote` 连接命令。
6. 宠物窗口监听 `codex-status-updated`，显示状态气泡。
7. 抽屉窗口显示当前 Codex 状态、连接方式、模式、错误消息和最近更新时间。

### 主题

1. 默认主题需要覆盖新增按钮、状态提示、输入框和错误提示样式。
2. `animal-island` 主题需要覆盖或确认继承：
   - Codex 设置面板
   - 状态徽标
   - 宠物气泡
   - 错误提示

## 隐私要求

1. 不把 Codex 消息、用户项目路径、终端输出、API Key、token、文件路径写入源码或文档示例。
2. 配置只能写入用户本机应用数据目录中的 `config.json`。
3. 状态事件只保留必要摘要，不持久化完整对话。
4. 日志和错误提示中避免输出原始用户任务内容。
5. 日志监听模式不持久化日志读取偏移，不保存会话日志内容，不展示本机真实日志路径。

## 验收标准

1. 本机安装 Codex CLI 且 Codex App 控制 socket 可用时，可以从设置页开始监听当前 Codex。
2. 在 Codex 中开始任务后，宠物进入 `running` 或 `review` 状态。
3. Windows Codex Desktop 写入会话日志时，`sessionLog` 模式能根据 `task_started` 进入 `running`，根据 `final_answer` 或 `task_complete` 显示完成提醒。
4. `managed` 模式触发测试任务后，宠物进入 `running` 状态。
5. App Server 返回完成事件后，宠物显示完成气泡并短暂切换完成动画。
6. App Server 报错时，宠物显示失败状态，设置页显示错误摘要。
7. 默认主题和 `animal-island` 主题下新增界面无明显重叠、溢出或不可读文本。
8. `npm run build` 通过。
9. 提交前隐私检查未发现真实密钥、本机配置或用户隐私数据进入仓库。
