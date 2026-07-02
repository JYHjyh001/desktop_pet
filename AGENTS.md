使用中文回答，每次回答告诉我详细步骤。

## 项目功能注意事项

- 每次新增或调整前端功能时，必须同时检查默认主题和 `animal-island` 主题的界面表现；新增窗口、弹窗、按钮、输入框、列表、空状态、错误提示都要有对应主题样式或确认可继承现有主题样式。
- 音乐播放功能中的歌曲路径、分类、播放列表、歌曲文件存储目录都属于用户本机数据；只能保存在本机运行时存储中，不允许写入源码、文档示例或提交到仓库。
- 音乐 metadata 标签读取只能在本机读取；当前音乐功能不接入音频指纹识别或外部识别服务，不允许上传原始音频、音频指纹、歌曲路径或识别服务密钥。
- 音乐添加功能需要同时考虑单文件添加、文件夹批量添加、分类归档和可选歌曲存储目录；如果设置了存储目录，后续导入歌曲应复制到用户选择的本机目录，不能覆盖同名文件。
- 后续逐步实现音乐功能时，先参考 `docs/music-feature-requirements.md` 中的阶段路线图和功能清单。
- Windows 下 `agent.md` 与 `AGENTS.md` 文件名大小写等同；当前项目统一使用根目录 `AGENTS.md` 作为后续对话的项目注意事项文件。

每次修改代码、打包程序、提交 Git、推送 GitHub 或发布 Release 前，必须先读取并执行：

- `docs/RELEASE_PRIVACY_CHECKLIST.md`

重点要求：

- 用户 AI API Key、Base URL、聊天配置、快捷入口、软件路径、文件夹路径、使用历史和导入的本地资源都只能作为用户本机数据保存。
- 不允许把本机 `config.json`、`apps.json`、`.env*`、证书、token、key、应用数据目录或打包缓存推送到 GitHub。
- 提交前必须检查 `git status --short`、`.gitignore` 和疑似密钥关键词扫描结果。

## 桌宠修改后重启规则

Windows 下不要用 `cargo build`、`cargo build --release` 或 `src-tauri\target\debug\pet_drawer.exe` 作为完整桌宠 UI 重启方式。

正确方式：

1. 执行 `npm run tauri:build`
2. 启动 `src-tauri\target\release\pet_drawer.exe`

原因：

- `src-tauri\target\debug\pet_drawer.exe` 会打开黑色终端窗口。
- 单独 `cargo build --release` 生成的 exe 可能仍访问 `127.0.0.1:1420` dev server，导致“拒绝连接”页面。
- `npm run tauri:build` 才会完整构建前端并由 Tauri 打包资源。
