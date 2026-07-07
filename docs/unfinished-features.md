# 未完成功能记录

本文档按模块记录计划继续完善或尚未实现的能力。用户本机路径、AI API Key、Base URL、聊天配置、音乐文件、导入素材、平台凭据、Codex 会话内容和运行历史不得写入本文档。

## 维护规则

1. 新待办先合并到既有模块；不要把同类事项拆成多条重复记录。
2. 已完成的事项应移动到 `docs/completed-features.md`，并在对应需求文档中更新状态。
3. 涉及用户数据、平台账号、音乐资源、AI 配置或 Codex 内容时，只记录隐私边界和通用规则。
4. 涉及前端功能时，验收必须包含默认主题和 `animal-island` 主题。

## 当前优先级总览

| 优先级 | 模块 | 当前结论 |
| --- | --- | --- |
| P0 | AI 宠物素材生成和转换 | 先做 `hatch-pet` 到 PetDrawer 的本机离线转换器，再做应用内 AI 生成入口。 |
| P1 | 工程维护优化 | 继续拆分超大前端窗口、复查注册命令和优化构建分包。 |
| P1 | Codex 状态提醒回归 | 已有代码级实现，需要在真实连接模式下回归并调优低打扰体验。 |
| P2 | 音乐状态与宠物联动 | 播放器基础已完成，下一步让音乐播放状态驱动宠物动作。 |
| P3 | 在线音乐平台增强 | 补播放兜底、歌单管理和诊断增强，不绕过平台限制。 |
| P4 | 低风险体验打磨 | 图标提取、宠物气泡、翻译可选能力和多显示器细节。 |

## P0：AI 宠物素材生成和转换

- [ ] 实现 `hatch-pet` / Codex pet atlas 到 PetDrawer 多状态动画包的本机离线转换器。
- [ ] 支持识别生成结果目录、动作行素材、`spritesheet`、`pet.json` 和可导入的状态文件。
- [ ] 输出 PetDrawer 可导入的动画文件和 manifest，不覆盖同名文件。
- [ ] 缺少可选状态时给出摘要并依赖现有回退规则，不让导入流程失败。
- [ ] 后续再增加应用内 AI 生成入口、生成进度、结果预览和失败处理。
- [ ] 参考文档：`docs/ai-pet-material-generation-requirements.md`。

## P1：工程维护优化

- [ ] 继续拆分 `src/windows/MusicWindow.vue`，优先把在线平台、沉浸模式、迷你播放器和本地库逻辑拆成 composable 或子组件。
- [ ] 继续拆分 `src-tauri/src/kugou_music.rs`、`src-tauri/src/ai_chat.rs` 和 `src/windows/DrawerWindow.vue` 等超大文件，降低单文件维护成本。
- [ ] 复查注册但没有直接字面量 `invoke` 的 Tauri 命令，确认是否属于动态调用、托盘调用、保留兼容或可删除接口。
- [ ] 继续优化构建分包；当前 WebGL 星河组件已异步加载，但主前端 chunk 和 Three.js chunk 仍超过 Vite 默认提示阈值。
- [ ] 建议新增常用检查脚本，封装 `npm run build`、严格未使用检查、`cargo check` 和隐私关键词扫描。
- [ ] 参考文档：`docs/project-code-audit-2026-07-07.md`。

## P1：Codex 状态提醒回归和体验调优

- [ ] 在真实 `proxy`、`managed`、`sessionLog` 三种模式下回归 Codex 状态事件链路。
- [ ] 继续验证多任务同时运行、等待用户处理、失败、完成和确认后的状态优先级。
- [ ] 评估低频重复提醒、声音提醒和更细聚合展示，但不恢复打扰式主动弹窗。
- [ ] 参考文档：`docs/codex-app-server-integration-requirements.md`、`docs/codex-multi-session-completion-notification-requirements.md`。

## P2：音乐播放状态与宠物动画联动

- [ ] 建立播放中、暂停、切歌、收藏、睡眠音乐、专注音乐等事件到宠物动画状态的映射。
- [ ] 增加是否启用音乐动画联动、是否允许 AI 主动推荐、是否允许 AI 自动播放等本机设置项。
- [ ] 先使用现有宠物状态回退规则，不要求每个宠物包都有专用音乐动画。
- [ ] 推荐逻辑只能使用标签、收藏、最近播放、播放次数等安全摘要，不上传歌曲路径、原始音频、音频指纹或歌词内容。
- [ ] 参考文档：`docs/music-feature-requirements.md`、`docs/music-immersive-mode-requirements.md`。

## P3：音乐体验和在线平台增强

- [ ] 本地音乐：继续完善歌单封面、批量维护、排序、标签分组、推荐解释、专注/睡眠/情绪模式。
- [ ] 沉浸音乐：评估 Butterchurn 主视觉、原生长音频解码回退、真实宠物动作联动和专注/睡眠/剧情氛围深度逻辑。
- [ ] 网易云：评估官方客户端或网页播放兜底、在线播放失败诊断增强。
- [ ] 酷狗：评估收藏管理、新建/删除歌单、增删歌单歌曲、官方客户端或网页播放兜底。
- [ ] 酷狗代理：继续评估官方客户端专用设备/风控授权差异和网易云同类代理复用；不得绕过会员、版权、地区、DRM 或付费限制。
- [ ] 参考文档：`docs/music-netease-cloud-requirements.md`、`docs/music-kugou-requirements.md`、`docs/music-kugou-playback-proxy-requirements.md`。

## P4：低风险体验打磨

- [ ] 抽屉图标提取后续增强：非 Windows 替代方案、多尺寸图标手动选择和更细分失败建议。
- [ ] 宠物气泡后续增强：更多气泡类型、手动位置偏好和更细粒度的多显示器体验调优。
- [ ] 翻译后续可选增强：历史记录、专用翻译 API、PDF/OCR 能力等需另起需求文档并重新确认隐私边界。
- [ ] 所有新增界面都要检查默认主题和 `animal-island` 主题。
