# 第三方许可与素材来源审计

审计日期：2026-07-13

本文档记录 PetDrawer 当前仓库的项目许可状态、直接依赖许可、随程序分发的素材来源线索和发布前待决事项。审计仅基于仓库文件、锁文件以及本机已安装依赖的元数据，不联网下载第三方内容，也不包含用户本机路径、音乐文件、导入素材、账号凭据或其他运行时隐私数据。

> 本文档是工程合规盘点，不构成法律意见，也不替项目所有者选择开源或闭源许可证。

## 审计范围与证据

- `package.json`、`package-lock.json` 和已安装 npm 包的 `package.json`：核对 npm 直接依赖的实际版本与许可证声明。
- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 和本机已有 crate 元数据：核对 Rust 直接依赖的实际版本与许可证声明。
- `src/assets/`、`src-tauri/icons/`、`scripts/create-icon.ps1`：盘点仓库随包素材及其来源线索。
- `README.md` 与 `docs/`：盘点明确提到的第三方设计、代码和服务参考。
- 本轮不对 npm、Cargo 的全部传递依赖做最终法律审查，也不验证在线音乐服务条款、商标权或每个司法辖区的分发要求。

## 结论摘要

| 等级 | 结论 | 发布前要求 |
| --- | --- | --- |
| 阻断 | 仓库没有根级 `LICENSE`，`package.json` 和 `src-tauri/Cargo.toml` 也没有项目许可证字段。`private: true` 只阻止误发 npm 包，不等于许可证。 | 项目所有者必须决定项目采用闭源条款、宽松开源许可或 copyleft 许可，并确认版权主体、年份、源码与二进制分发范围。 |
| 高 | Release 构建现已移除仓库外图标覆盖，并用固定 SHA-256 校验仓库内 ICO/ICNS；但当前图标的作者、来源、授权和商标使用权仍未登记。 | 项目所有者确认权利来源后更新受控摘要；任何获批图标变更必须同时更新文件和校验摘要。 |
| 中 | 35 个 Twemoji SVG 明确采用 CC-BY 4.0。现已增加根级 notices、扩展素材 NOTICE，并配置为 Tauri bundle resources。 | 完整打包后验证 MSI/NSIS 安装结果确实包含 notices，且应用内署名在离线环境可访问。 |
| 高 | 当前没有覆盖完整 npm/Cargo 传递依赖的发行版 notices 清单。 | 在正式 Release 前生成完整依赖物料清单，并人工复核非 MIT/Apache 条目、许可证文件和原生组件。 |
| 中 | 3 个默认入口图标和 5 个默认宠物 SVG 没有第三方声明，也没有仓库内的作者/来源记录。 | 由项目所有者确认这些素材是原创、委托且已转让、或已取得可分发授权，并保存来源证明。 |
| 中 | 文档说明参考过 Mineradio、Sonic Topography、animal-island-ui 和 KuGouMusicApi 等项目，但没有把所有上游许可证快照和对照结论固定到仓库。 | 发布前做一次人工来源对照，确认没有复制受限代码、shader、素材、Logo、命名或具有独创性的视觉表达。 |

## 项目自身许可证：待用户决定

当前状态：

- 仓库根目录没有 `LICENSE`、项目级 `NOTICE` 或版权声明文件。
- `package.json` 没有 `license` 字段，且 `private: true` 只影响 npm 发布行为。
- `src-tauri/Cargo.toml` 没有 `license` 或 `license-file` 字段。
- 因此，外部用户无法从仓库判断能否复制、修改、再分发或商用；二进制接收者也没有项目自身的使用条款。

项目所有者需要明确选择，不应由开发代理代选：

1. **闭源或仅保留所有权**：确定源码可见范围、终端用户许可协议、商业使用、再分发、逆向工程、隐私条款和免责声明；第三方依赖与 CC-BY 素材义务仍须履行。
2. **宽松开源许可**：在 MIT、Apache-2.0、双许可或其他许可之间决定是否需要专利授权、NOTICE 机制及再分发条件，并确定准确版权主体与年份。
3. **Copyleft 许可**：决定是否要求衍生作品或网络服务公开源码，评估桌面二进制、插件、素材和第三方服务接入的边界，并建立提供对应源码和许可证文本的发布流程。

在项目所有者完成选择前，不能把“许可证和第三方许可清单”标记为完全完成。

## npm 直接运行依赖

以下版本来自当前锁文件和已安装包元数据；`^` 范围本身不是发布时的实际版本。

| 包 | 实际版本 | 上游声明 | 主要用途 | 发行注意事项 |
| --- | --- | --- | --- | --- |
| `@tauri-apps/api` | 2.11.0 | Apache-2.0 OR MIT | 前端调用 Tauri 窗口、事件和 IPC API | 按选定许可路径保留上游许可证及版权声明。 |
| `@tauri-apps/plugin-dialog` | 2.7.1 | MIT OR Apache-2.0 | 本机文件和目录选择器 | 同上。 |
| `@tauri-apps/plugin-global-shortcut` | 2.3.2 | MIT OR Apache-2.0 | 全局快捷键前端接口 | 同上。 |
| `three` | 0.185.0 | MIT | WebGL 星河、歌词舞台和音乐可视化 | 分发实质性代码时保留 MIT 许可证与版权声明。 |
| `vue` | 3.5.34 | MIT | 前端 UI 框架 | 分发实质性代码时保留 MIT 许可证与版权声明。 |

直接运行依赖均为宽松许可证声明，未发现直接 GPL/AGPL npm 依赖。但这不替代对传递依赖和实际打包文件的检查。

## npm 直接开发依赖

开发依赖通常不随最终程序运行，但构建工具产生的代码、注入的 runtime helper 或被复制进安装包的文件仍需按最终产物复核。

| 包 | 实际版本 | 上游声明 | 主要用途 |
| --- | --- | --- | --- |
| `@tauri-apps/cli` | 2.11.2 | Apache-2.0 OR MIT | Tauri 开发、构建和安装包生成 |
| `@types/three` | 0.185.0 | MIT | Three.js TypeScript 类型 |
| `@vitejs/plugin-vue` | 5.2.4 | MIT | Vue 单文件组件构建插件 |
| `@vue/test-utils` | 2.4.6 | MIT | Vue 组件测试 |
| `jsdom` | 24.1.3 | MIT | 前端 DOM 测试环境 |
| `typescript` | 5.9.3 | Apache-2.0 | TypeScript 编译与类型检查 |
| `vite` | 5.4.21 | MIT | 前端开发与生产构建 |
| `vitest` | 3.2.6 | MIT | 前端测试运行器 |
| `vue-tsc` | 2.2.12 | MIT | Vue TypeScript 类型检查 |

## Rust 直接依赖

以下是 `src-tauri/Cargo.lock` 中由 `pet_drawer` 直接引用的解析版本。`tauri-build` 是构建依赖，其余是运行或目标平台依赖。

| crate | 实际版本 | 上游声明 | 主要用途 |
| --- | --- | --- | --- |
| `tauri-build` | 2.6.2 | Apache-2.0 OR MIT | Tauri 构建脚本 |
| `base64` | 0.22.1 | MIT OR Apache-2.0 | 平台协议与二进制编码 |
| `aes` | 0.8.4 | MIT OR Apache-2.0 | 酷狗协议所需本机加密处理 |
| `cbc` | 0.1.2 | MIT OR Apache-2.0 | 分组密码 CBC 模式 |
| `cipher` | 0.4.4 | MIT OR Apache-2.0 | 加密算法通用接口和填充 |
| `ecb` | 0.1.2 | MIT | 分组密码 ECB 模式 |
| `md-5` | 0.10.6 | MIT OR Apache-2.0 | 平台请求签名摘要 |
| `num-bigint` | 0.4.6 | MIT OR Apache-2.0 | 大整数协议计算 |
| `qrcode` | 0.14.1 | MIT OR Apache-2.0 | 登录二维码生成 |
| `reqwest` | 0.12.28 | MIT OR Apache-2.0 | HTTP 请求 |
| `serde` | 1.0.228 | MIT OR Apache-2.0 | 配置和命令数据序列化 |
| `serde_json` | 1.0.149 | MIT OR Apache-2.0 | JSON 读写 |
| `rusqlite` | 0.32.1 | MIT | 本机记忆数据库；启用了 bundled SQLite 特性 |
| `tauri` | 2.11.2 | Apache-2.0 OR MIT | 桌面应用运行时 |
| `tauri-plugin-dialog` | 2.7.1 | Apache-2.0 OR MIT | 原生文件和目录对话框 |
| `tauri-plugin-global-shortcut` | 2.3.2 | Apache-2.0 OR MIT | 系统全局快捷键 |
| `lofty` | 0.24.0 | MIT OR Apache-2.0 | 本机音乐 metadata 标签读取 |
| `tungstenite` | 0.24.0 | MIT OR Apache-2.0 | Codex app-server WebSocket 通信 |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | Windows 窗口、剪贴板、进程、Shell 和输入 API |

直接 Rust 依赖均为 MIT、Apache-2.0 或二者择一。发布清单仍需覆盖：

- `rusqlite` 的 bundled SQLite 原生组件及其构建链；
- Tauri、WebView2、NSIS/WiX 等实际进入安装链或安装包的组件与分发条款；
- Rust 传递依赖中可能出现的 BSD、ISC、Zlib、Unicode、MPL 或其他许可；
- Apache-2.0 依赖若带上游 `NOTICE`，最终发行版是否需要同步保留。

## 仓库素材盘点

### Twemoji

- 位置：`src/assets/emoji/twemoji/svg/`
- 数量：35 个 SVG。
- 仓库声明：`src/assets/emoji/twemoji/NOTICE.md` 标注图形来自 Twemoji，采用 CC-BY 4.0，并给出上游仓库地址。
- 使用位置：`src/utils/twemoji.ts` 将这些 SVG 作为聊天展示和表情选择器资源打包。
- 当前缺口：Vite 只会打包被引用的 SVG，不能据此认为未被引用的 `NOTICE.md` 也会进入桌面安装包。

发布时至少应提供：作品/项目名称、Twemoji 来源、CC-BY 4.0 标识与链接、适当的作者或项目署名，以及项目是否修改过图形的说明。最终展示位置应让离线安装后的用户能够访问，例如应用内“第三方许可”页面或明确打包的 notices 文件。

### 默认入口图标与默认宠物

| 素材 | 数量 | 当前证据 | 风险 |
| --- | --- | --- | --- |
| `src/assets/icons/*.svg` | 3 | 文件内没有第三方版权、作者或许可注释 | 无法仅凭仓库证明原创或已获授权。 |
| `src/assets/pets/default/*.svg` | 5 | 文件内没有第三方版权、作者或许可注释 | 无法仅凭仓库证明原创或已获授权。 |
| `src/assets/pets/default/pet.json` | 1 | 仅描述默认宠物状态映射 | 应与对应 SVG 作为同一素材包确认来源。 |

建议增加简短的素材来源登记，至少记录“原创/委托/第三方”、作者或权利人、创建日期、授权范围和证据存放位置。若是项目原创，可以在项目许可证选择后明确素材是否随代码采用同一许可。

### 应用图标

`scripts/create-icon.ps1` 已改为只验证仓库内受控的 `icon.ico` 与 `icon.icns`：

- 不再读取、复制或探测仓库外图标；
- 使用兼容 Windows PowerShell 的 .NET SHA-256 实现核对固定摘要；
- `npm run check:icons` 已加入本机聚合检查和 Windows CI；
- 文件缺失或摘要变化会在构建前失败，避免同一提交因开发机外部文件不同而生成不同图标。

这解决了构建输入不封闭的问题，但没有自动证明现有图标的版权和商标授权。正式发布前仍需由项目所有者登记当前 ICO/ICNS 的作者、来源、商业分发范围和证据位置；后续合法替换图标时，应在人工审核后同时更新受控文件与脚本摘要。

### 字体、音频、视频与 3D 模型

本轮仓库扫描没有发现被跟踪的 `ttf`、`otf`、`woff`、`woff2`、音频、视频或 3D 模型文件。CSS 使用字体栈和系统字体回退，不等于仓库重新分发字体文件。

用户导入的音乐、壁纸、图标和宠物素材属于本机运行时数据，不是仓库随包素材；应用也不应把这些文件再次分发或写入第三方 notices。

## 第三方参考与服务边界

下列项目没有作为直接 npm/Cargo 依赖出现，但文档明确提到过参考：

| 参考对象 | 仓库内记录 | 当前判断 | 仍需人工确认 |
| --- | --- | --- | --- |
| Mineradio | 多份沉浸音乐需求文档声明其为 GPL-3.0，并反复约束不复制源码、shader、素材、Logo、命名和原始视觉表达 | 目前只看到“高层原则、自研实现”的项目记录，未发现仓库内携带其素材或许可证文件 | 对视觉结构、shader、参数表和交互细节做来源对照；若存在复制，必须按上游许可重新评估整个分发方案。 |
| Sonic Topography | 地面 DJ 文档记录参考 GPU 地形、频段触发和材质分层原则，并声明不复制源码或素材 | 仓库没有固定上游许可证快照 | 核对上游准确许可证和对应 commit，确认实现没有复制代码、shader、资源、字体或独特参数表。 |
| `guokaigdg/animal-island-ui` | `docs/animal-island-theme-refresh-requirements.md` 固定了参考 commit，并说明不引入 React 组件或大量上游资产 | 主要是主题 token、字体栈和组件风格参考 | 核对上游许可证，并确认没有逐段复制受限 CSS、图片、字体或演示资产。 |
| `MakcRe/KuGouMusicApi` | 酷狗需求文档记录参考接口形态 | 不是 Cargo/npm 依赖，且许可证未固定到仓库 | 核对上游许可、复制边界和 API 服务条款；接口事实与实现代码应分开审查。 |

网易云、酷狗等在线服务的接口使用、账号登录、播放链接、歌词、专辑封面、商标和内容版权不由 npm/Cargo 许可证解决。正式公开分发前还需要独立确认服务条款、地区限制、内容展示和商标使用规则；不得把用户 Cookie、token、设备标识、播放 URL 或本机音乐路径写入该审计或发行物料。

## 发布前推荐实施顺序

1. **用户决定项目许可证**：明确版权主体、年份、源码授权、二进制分发、商业使用、专利与 copyleft 选择。
2. **补齐图标权属**：构建输入已经封闭并带摘要校验；继续确认 Release 图标的作者、来源、授权和商标使用权。
3. **建立完整 SBOM/许可清单**：从锁文件枚举 npm 与 Cargo 全部传递依赖，标记运行时、构建时和目标平台条件依赖。
4. **生成发行版 notices**：生成项目级第三方 notices；不要在审计文档中复制整份许可证正文，但发行包应按上游条款携带所需完整许可证和版权声明。
5. **验证 Twemoji 署名**：根级 notices、素材 NOTICE 和 Tauri resource 配置已建立；继续检查 MSI、NSIS 安装结果和应用内离线可访问性。
6. **补自有素材来源登记**：确认默认 SVG、宠物包和图标的权利归属。
7. **人工复核参考实现**：重点对 Mineradio、Sonic Topography、animal-island-ui 和 KuGouMusicApi 做代码/视觉来源对照。
8. **加入 CI 门禁**：后续可评估锁定版本的 npm 许可扫描和 `cargo-deny`/同类检查；新出现未知、强 copyleft 或缺失许可证的依赖必须人工批准。

## 完成判定

只有同时满足以下条件，许可项才可以从“未完成”移入“已完成”：

- [ ] 项目所有者已选择并落地项目许可证或闭源使用条款。
- [ ] 根级许可证、包元数据与应用内/安装包展示相互一致。
- [ ] 应用图标和默认 SVG 素材都有可验证来源与分发授权。
- [ ] Twemoji CC-BY 4.0 署名能够在最终安装后的应用中访问。
- [ ] npm/Cargo 全部传递依赖已经过许可扫描和人工例外复核。
- [ ] 最终发行包携带所需许可证、版权声明和 notices。
- [ ] 第三方参考实现完成来源对照，没有未处理的 GPL 或视觉表达复制风险。
- [ ] 发布前重新执行隐私扫描，许可物料中不包含用户配置、密钥、路径、音乐或导入素材。
