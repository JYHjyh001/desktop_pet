# Release Privacy Checklist

每次修改代码、打包程序、提交 Git、推送 GitHub 或发布 Release 前，都必须先阅读并执行本清单。

## 本地隐私数据规则

1. 用户 AI API Key、Base URL、模型配置、系统提示词和聊天相关配置只允许保存在用户本机应用数据目录的 `config.json`。
2. 用户的软件路径、文件夹路径、网站入口、打开历史和本地导入资源只允许保存在用户本机应用数据目录的 `apps.json`、`storage.json`、`icons/`、`pets/` 等目录。
3. 音乐路径、分类、播放列表、歌词来源和平台登录态只允许保存在用户本机应用数据目录的 `music-library.json`、`music-platforms/` 等运行时文件中；不得写入源码、文档示例或提交到仓库。
4. 自定义数据目录中的同名文件同样属于本机隐私数据，忽略规则不能只覆盖仓库根目录。
5. 调用在线 AI 前，界面必须明确说明文本会发送到用户配置的服务；不得上传本地音频、音频指纹、歌曲路径或本地资源。
6. 不允许将真实用户配置、真实 API Key、真实 token、证书、私钥、`.env*`、本机应用数据或打包缓存提交到 Git。
7. README 和代码示例只能使用占位说明，不允许写入真实密钥。
8. Windows 配置密钥使用当前用户 DPAPI 加密；加密后的 config.json 仍属于本机隐私数据，不得提交。旧明文配置只读兼容，自动位置保存不迁移旧密钥，明确保存设置时升级；解密/加密失败保留原文件，不回退明文或生成明文备份。换机器、换 Windows 用户或降级前须明确兼容影响。
9. `chat-attachments/` 是用户本机聊天附件，和记忆目录一起管理，不得提交或上传。附件有配额，历史消息删除不自动清理附件；导出时仅在用户主动操作下将引用展开为本机 JSON。新旧图文历史不自动批量重写或裁剪。

## 提交前必须检查

1. 读取 `.gitignore`，确认至少忽略：
   - `.env`、`.env.*`
   - 任意层级的 `config.json`、`apps.json`、`storage.json`
   - `pet-memory.json`、`pet-memory.db`、`pet-memory.db-*`、记忆导出文件
   - `music-library.json`、`music-platforms/`、`netease-cloud*.json`、`kugou*.json`
   - `PetDrawer/`、`app-data/`、`data/`、`user-data/`、`local-data/`
   - `src-tauri/target/`、`dist/`、打包产物和日志
   - `*.key`、`*.pem`、`*.p12`、`*.pfx`、`*.token`、`*.credential*`
2. 执行 `git status --short`，确认没有本机配置文件或私密文件出现在待提交列表。
3. 执行关键词扫描，至少覆盖：
   - `apiKey`
   - `api_key`
   - `OPENAI`
   - `DEEPSEEK`
   - `ANTHROPIC`
   - `GEMINI`
   - `sk-`
   - `AIza`
   - `token`
   - `secret`
   - `credential`
4. 如果扫描命中的是字段名、代码逻辑或文档占位说明，可以提交；如果命中真实密钥或本机路径，必须先移除。

## Release 前必须检查

1. 版本号需要同步更新：
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
   - 必要时同步 `src-tauri/Cargo.lock` 中本项目包版本。
2. Release 附件只能上传构建产物，不上传源码外的本机配置。
3. Release notes 必须说明 AI API Key 等配置只保存在用户本机，不会包含在仓库或 Release 附件中。
4. 推送前再次确认 `git status --short` 中只包含必要代码、文档和配置模板变更。
5. 执行 `npm run check:icons`，确认构建只使用仓库内已审核且摘要匹配的应用图标，不读取开发机外部品牌素材。
6. 检查最终安装包中的 `THIRD_PARTY_NOTICES.md` 和素材 notices，确保许可物料不包含用户配置、本机路径或运行历史。
