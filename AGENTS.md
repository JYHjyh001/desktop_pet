使用中文回答，每次回答告诉我详细步骤。

每次修改代码、打包程序、提交 Git、推送 GitHub 或发布 Release 前，必须先读取并执行：

- `docs/RELEASE_PRIVACY_CHECKLIST.md`

重点要求：

- 用户 AI API Key、Base URL、聊天配置、快捷入口、软件路径、文件夹路径、使用历史和导入的本地资源都只能作为用户本机数据保存。
- 不允许把本机 `config.json`、`apps.json`、`.env*`、证书、token、key、应用数据目录或打包缓存推送到 GitHub。
- 提交前必须检查 `git status --short`、`.gitignore` 和疑似密钥关键词扫描结果。
