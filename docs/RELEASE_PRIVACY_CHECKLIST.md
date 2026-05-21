# Release Privacy Checklist

每次修改代码、打包程序、提交 Git、推送 GitHub 或发布 Release 前，都必须先阅读并执行本清单。

## 本地隐私数据规则

1. 用户 AI API Key、Base URL、模型配置、系统提示词和聊天相关配置只允许保存在用户本机应用数据目录的 `config.json`。
2. 用户的软件路径、文件夹路径、网站入口、打开历史和本地导入资源只允许保存在用户本机应用数据目录的 `apps.json`、`icons/`、`pets/` 等目录。
3. 不允许将真实用户配置、真实 API Key、真实 token、证书、私钥、`.env*`、本机应用数据或打包缓存提交到 Git。
4. README 和代码示例只能使用占位说明，不允许写入真实密钥。

## 提交前必须检查

1. 读取 `.gitignore`，确认至少忽略：
   - `.env`、`.env.*`
   - `config.json`、`apps.json`
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
