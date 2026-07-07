# 项目代码扫描与优化记录（2026-07-07）

本文档记录本轮对整个项目的代码、构建配置和开发文档扫描结果。本文档不记录用户本机路径、AI API Key、Base URL、聊天配置、音乐文件、导入素材、平台凭据、Codex 会话内容或运行历史。

## 扫描范围

- 前端：`src/**/*.vue`、`src/**/*.ts`、`vite.config.ts`、`package.json`。
- 后端：`src-tauri/src/**/*.rs`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`。
- 文档：`README.md`、`AGENTS.md`、`docs/*.md`、`docs/releases/*.md`。
- 项目规则：`.gitignore`、`docs/RELEASE_PRIVACY_CHECKLIST.md`。

## 已执行检查

| 检查 | 结果 |
| --- | --- |
| `git status --short` 初始检查 | 开始扫描前工作区干净。 |
| `npm run build` | 通过。 |
| `npx vue-tsc --noEmit --noUnusedLocals --noUnusedParameters` | 通过；本轮已清理发现的未使用前端符号。 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 通过。 |
| `rg "TODO|FIXME|HACK|console\\.log|debugger" src src-tauri/src` | 未发现命中。 |
| 前端 `invoke` 与 Tauri handler 粗对照 | 未发现“前端调用但未注册”的命令。 |
| 文档索引扫描 | 顶层 `docs` 文档较多，音乐相关需求文档占多数；本轮已整理入口文档。 |
| 疑似密钥关键词扫描 | 广义扫描仅命中文档规则、占位说明、字段名和代码变量名；高风险 `sk-` / `AIza` 前缀扫描未发现命中。 |
| `git diff --check` | 通过。 |

## 本轮已完成优化

- 清理 `MusicWebglStarfield.vue` 中未引用的 `clearTerrainCenterPulse`。
- 清理 `defaultPet.ts` 中未使用的类型导入。
- 清理 `DrawerWindow.vue` 中未使用的图片选择函数。
- 清理 `MusicWindow.vue` 中已不再渲染的场景歌单/AI 推荐选项、说明函数和视觉标签计算属性。
- 清理 `PetChatWindow.vue` 中不再使用的本地音乐意图构造函数。
- 优化 `syncCustomPlaylists()`，只有在曲目 ID 实际被清理时才写回自定义歌单数组。
- 统一开发端口：`package.json`、`src-tauri/tauri.conf.json` 和 `vite.config.ts` 均指向 `127.0.0.1:1421`。
- 将 `MusicWebglStarfield.vue` 改为 Vue 异步组件，避免沉浸 WebGL 组件代码直接进入主前端 chunk。
- 同步更新 `AGENTS.md` 中关于 dev server 端口的说明。
- 将 `completed-features.md`、`unfinished-features.md` 和 `next-development-plan.md` 整理为模块化开发文档。

## 构建体积观察

本轮优化前，前端生产构建中主 JS chunk 约 625 KB，Three.js chunk 约 735 KB。

将 `MusicWebglStarfield.vue` 异步加载后：

- 主 JS chunk 约 556 KB。
- 新增 `MusicWebglStarfield` 异步 chunk 约 68 KB。
- Three.js 仍为独立 chunk，约 735 KB。

Vite 仍会提示部分 chunk 超过 500 KB。当前 Three.js 属于沉浸 WebGL 功能的必要依赖，已经通过动态导入和异步组件降低首屏主包压力。后续如果继续压缩，需要进一步拆分 `MusicWindow.vue` 或配置更细的 manual chunks。

## 大文件和维护风险

当前最高行数文件：

| 文件 | 约行数 | 建议 |
| --- | ---: | --- |
| `src/windows/MusicWindow.vue` | 12310 | 优先拆分在线平台、沉浸模式、迷你播放器、本地库和设置逻辑。 |
| `src-tauri/src/kugou_music.rs` | 5769 | 按登录、搜索、歌单、播放 URL、代理和音质诊断拆分模块。 |
| `src/windows/DrawerWindow.vue` | 4110 | 拆分设置页、伴侣管理、宠物素材管理和 Codex 设置区。 |
| `src-tauri/src/ai_chat.rs` | 3773 | 拆分通用 AI 请求、宠物聊天、音乐意图、翻译和测试连接。 |
| `src/components/MusicWebglStarfield.vue` | 3340 | 后续可拆分歌词纹理、地面 DJ 地形、星河粒子和相机控制。 |
| `src-tauri/src/app_data.rs` | 2659 | 后续可拆分配置、快捷入口、宠物素材、存储迁移和音乐数据路径。 |

## 命令注册观察

粗略扫描显示：

- 前端可识别的直接 `invoke` 命令约 112 个。
- Tauri 注册 handler 约 120 个。
- 未发现前端直接调用但未注册的命令。
- 少量注册命令没有被简单字面量扫描识别到，原因可能是动态命令字符串、托盘直接调用、兼容保留或确实已废弃。

后续清理时需要逐个确认，不能只凭脚本结果删除。当前优先复查：

- `set_quick_search_tags`
- `import_pet_image`
- `reset_pet_image`
- `get_story_save`
- `show_pet`

其中 `show_pet` 已被系统托盘通过 Rust 直接调用，不能按前端未 invoke 判断为废弃。

## 文档整理结果

- `docs/completed-features.md`：从长流水记录改为模块化完成能力索引。
- `docs/unfinished-features.md`：按 P0-P4 优先级合并同类未完成项。
- `docs/next-development-plan.md`：同步当前基线、推荐路线、阶段目标和验收标准。
- 本文档记录本轮代码扫描、已处理优化和后续技术债。

## 后续建议

1. 下一轮优先拆分 `MusicWindow.vue`，这是目前最大的维护风险。
2. 为常用检查新增脚本，例如 `check:frontend:strict`、`check:rust`、`check:privacy`，减少手动漏检。
3. 注册命令清理要结合前端动态调用、托盘、窗口生命周期和兼容需求逐个验证。
4. 继续保持音乐路径、平台凭据、AI 配置、导入素材和 Codex 内容只作为用户本机数据处理。
