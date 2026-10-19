# AI 宠物素材生成需求计划

本文档用于规划“通过图片或文字生成 PetDrawer 可用 AI 宠物素材”的长期实现路线，并记录本机离线转换器的实施与验收边界。

## 当前实施范围：AI 宠物素材离线转换器

本阶段只处理用户已经在本机生成好的 `hatch-pet` 素材，不在 PetDrawer 内调用图片生成服务：

- 选择一个 `hatch-pet` 运行目录或宠物包目录。
- 自动优先识别 v2 `1536 x 2288`、`8 x 11` 图集，并兼容旧版 `1536 x 1872`、`8 x 9` 图集。
- 校验图集尺寸、已使用单元格、透明背景和可用 QA 证据；结构校验通过不等同于视觉 QA 已通过。
- 把 9 个标准动作行离线裁切并编码为 PetDrawer 可播放的透明 GIF。
- 按现有语义映射为 12 个 PetDrawer 动画状态，先预览再通过现有导入流程复制到本机宠物素材目录。
- 转换中间文件只进入系统临时目录，成功导入或取消时清理；不覆盖用户源素材。
- 不上传图集、路径、QA 文件或动画，不读取 AI API Key，不写入源码和日志。

应用内 AI 生成、参考图上传和第三方生成服务仍属于后续阶段。

## 实现原则

- 优先使用可复用 Codex skill 生成素材，不把一次性脚本当作完整功能。
- 用户上传或选择的参考图、生成的中间图、宠物素材包和导入路径都属于用户本机数据，不能写入源码、示例文档或仓库。
- AI API Key、Base URL、第三方服务 token 和生成服务配置只能保存在用户本机配置中，不能进入仓库。
- 图片生成优先复用已安装的 `hatch-pet` 和 `imagegen` skill；GIF 预览可复用 `create-gif-from-images` skill。
- 每次新增或调整前端入口、弹窗、按钮、输入框、列表、空状态和错误提示时，必须同时检查默认主题和 `animal-island` 主题。
- 先打通“生成素材 -> 转换格式 -> 导入 PetDrawer”的稳定流程，再考虑应用内一键生成和第三方生成服务。

## 当前项目状态

### 已有能力

- [x] PetDrawer 支持规范化宠物形象库。
- [x] 当前宠物形象由 `pet.json` 描述。
- [x] 当前项目内置默认宠物已包含多状态动画字段；新增状态当前可使用内置基础动画作为回退资源。
- [x] 本地导入宠物时，至少需要选择 `idle` 待机动画。
- [x] 导入后的宠物素材会复制到用户本机应用数据目录，不应提交到仓库。
- [x] 支持导入 `png`、`jpg`、`jpeg`、`webp`、`gif`、`ico`、`webm`、`mp4` 等素材格式。
- [x] 当前 UI 已有“更换宠物形象 / 导入宠物 / 编辑动画”入口。

### 已确认可用的相关 skills

- [x] `hatch-pet`：可从角色图、参考图、品牌线索或文字概念生成 Codex 兼容宠物素材。
- [x] `imagegen`：可生成或编辑位图素材，作为 `hatch-pet` 的主要图片生成能力。
- [x] `create-gif-from-images`：可把图片帧、spritesheet 或截图整理为 GIF 预览。

### 主要差异

- `hatch-pet` 当前正式输出是 Codex v2 pet atlas：前 9 行是标准动作，后 2 行是 16 个注视方向。
- PetDrawer 当前运行时已支持 `idle`、`hover`、`click`、`dragging`、`draggingLeft`、`draggingRight`、`waving`、`jumping`、`waiting`、`running`、`review`、`failed`。
- 因此后续重点不再是扩展基础状态模型，而是把 `hatch-pet` 的 atlas、row strip 或生成目录稳定转换成 PetDrawer 可导入的多状态素材包。

## hatch-pet 输出规格

`hatch-pet` 的准备脚本会生成一次宠物制作运行目录，核心结构包括：

- `pet_request.json`：本次宠物生成请求、atlas 尺寸、动作行和风格约束。
- `imagegen-jobs.json`：图片生成任务清单。
- `prompts/base-pet.md`：基础宠物形象提示词。
- `prompts/rows/*.md`：各动作行提示词。
- `prompts/row-retries/*.md`：动作行失败后的重试提示词。
- `references/layout-guides/*.png`：每个动作状态的布局参考图。
- `decoded/*.png`：生成后选定的基础图和各动作行图片。
- `qa/`：后续 QA 图、预览和检查结果目录。

### Atlas 尺寸

- 列数：8。
- 正式 v2 行数：11；旧版兼容输入为 9 行。
- 单格尺寸：`192 x 208`。
- 正式 v2 atlas 尺寸：`1536 x 2288`。
- 旧版/中间 atlas 尺寸：`1536 x 1872`，缺少 16 个注视方向，转换器必须给出兼容警告。
- 背景处理：使用可移除纯色 chroma key，再转为透明背景。

### 动作状态

| hatch-pet 状态 | 行号 | 帧数 | 用途 |
|---|---:|---:|---|
| `idle` | 0 | 6 | 安静待机、呼吸、眨眼 |
| `running-right` | 1 | 8 | 向右拖动或移动 |
| `running-left` | 2 | 8 | 向左拖动或移动 |
| `waving` | 3 | 4 | 打招呼或吸引注意 |
| `jumping` | 4 | 5 | 跳跃、轻盈悬停 |
| `failed` | 5 | 8 | 失败、阻塞、取消反馈 |
| `waiting` | 6 | 6 | 等待用户输入、批准或帮助 |
| `running` | 7 | 6 | 正在工作、思考、处理任务 |
| `review` | 8 | 6 | 检查完成结果、准备展示 |

v2 额外方向行固定为：

- 第 9 行：`000`、`022.5`、`045`、`067.5`、`090`、`112.5`、`135`、`157.5`。
- 第 10 行：`180`、`202.5`、`225`、`247.5`、`270`、`292.5`、`315`、`337.5`。

PetDrawer 当前尚未直接播放指针注视方向，因此离线转换器保留 v2 识别与 QA 摘要，但只转换前 9 行标准动作。方向跟随播放另列后续运行时能力。

### 生成约束

- `base` 可以只用文字生成。
- 每个动作行必须基于 `canonical-base.png` 保持身份一致。
- `running-left` 可以在确认安全后由 `running-right` 水平镜像生成，否则需要单独生成。
- 每个动作行都需要有布局参考，不能把参考线、格子、文字或标记生成进最终素材。
- 宠物必须保持小尺寸可读：完整身体、清晰轮廓、稳定脸部、稳定配色、无文字、无水印。

## PetDrawer 目标素材规格

### 当前兼容格式

PetDrawer 当前自定义宠物 `pet.json` 的核心结构应继续保持简洁：

```json
{
  "id": "skin_占位",
  "name": "宠物名称",
  "animations": {
    "idle": "idle.gif",
    "hover": "hover.gif",
    "click": "click.gif",
    "dragging": "dragging.gif",
    "draggingLeft": "dragging-left.gif",
    "draggingRight": "dragging-right.gif",
    "waving": "waving.gif",
    "jumping": "jumping.gif",
    "waiting": "waiting.gif",
    "running": "running.gif",
    "review": "review.gif",
    "failed": "failed.gif"
  },
  "createdAt": "占位时间"
}
```

### 状态映射建议

MVP 阶段优先把 `hatch-pet` 的 9 个状态转换成 PetDrawer 已支持的多状态字段：

- `idle` -> `idle`
- `waving` 或 `review` -> `hover`
- `jumping` 或 `waving` -> `click`
- `running-right` -> `dragging`
- `running-left` -> `draggingLeft`
- `running-right` -> `draggingRight`
- `waving` -> `waving`
- `jumping` -> `jumping`
- `waiting` -> `waiting`
- `running` -> `running`
- `review` -> `review`
- `failed` -> `failed`

如果缺少某个可选状态，转换器应在摘要中说明并依赖 PetDrawer 现有回退规则，不让导入流程失败。AI 对话和音乐播放的状态联动仍是后续运行时接入工作。

## 功能目标

### MVP：外部生成工作流

目标：让用户可以在 Codex 中通过参考图生成宠物素材，并导入 PetDrawer 使用。

- [ ] 用户提供参考图片或文字描述。
- [ ] 使用 `hatch-pet` 生成基础宠物和动作行。
- [ ] 生成完成后得到 contact sheet、motion preview、spritesheet 和检查结果。
- [x] 增加转换流程，把 Codex pet atlas 转换为 PetDrawer 当前可读取的多状态动画文件，至少包含 `idle`。
- [x] 生成本机可导入的 `pet.json`。
- [x] 转换结果自动填入现有导入表单，用户确认后导入并使用。
- [x] 转换素材只保存在系统临时目录和用户本机宠物目录，不写入源码目录。

### 第二阶段：PetDrawer 内部导入优化

目标：减少用户手动选择多个动画文件的步骤。

- [x] 支持选择一个生成结果目录。
- [x] 自动识别 `pet.json`、`pet_request.json`、标准/扩展 `spritesheet` 和 QA 状态文件。
- [x] 自动填入宠物名称和转换后的动画路径。
- [x] 支持待机动画预览，并显示图集版本、结构检查、QA 证据和兼容警告；逐状态大图预览继续复用导入后的现有形象详情面板。
- [ ] 如果缺少 `hover`、`click` 或 `dragging`，允许使用 `idle` 回退。
- [x] 显示导入错误：目录/图集缺失、尺寸不符、必须帧为空、未使用格不透明、素材无法读取或编码失败。
- [x] 默认主题和 `animal-island` 主题都已接入转换入口、预览卡片、隐私提示、警告和禁用状态，并有双主题结构回归测试。

### 第三阶段：应用内 AI 生成入口

目标：在 PetDrawer 中提供“生成宠物素材”的入口，但不牺牲隐私和可控性。

- [ ] 在“更换宠物形象”窗口中增加“AI 生成”入口。
- [ ] 支持输入宠物名称、描述、风格和参考图。
- [ ] 明确提示参考图和生成结果会作为本机用户数据处理。
- [ ] 允许用户选择生成输出目录。
- [ ] 只在用户明确确认后调用生成流程。
- [ ] 生成服务配置、API Key、token 只能保存在本机配置，不写入仓库。
- [ ] 生成失败时显示可理解的错误，不泄漏密钥、token 或完整本机路径。
- [ ] 默认主题和 `animal-island` 主题都要适配完整弹窗。

### 第四阶段：多状态宠物联动

目标：让完整 9 状态 atlas 真正服务桌宠行为。

- [x] 扩展 PetDrawer 的宠物状态模型。
- [ ] 支持 AI 聊天等待时显示 `waiting`。
- [ ] 支持 AI 请求处理中显示 `running`。
- [ ] 支持 AI 回复完成或结果检查时显示 `review`。
- [ ] 支持请求失败、取消或阻塞时显示 `failed`。
- [x] 支持拖动方向区分 `running-left` 和 `running-right`。
- [ ] 支持音乐播放或节奏模式映射到 `jumping` 或其他动作。
- [x] 保持旧版 4 状态宠物素材可继续使用。

## 隐私和安全要求

- 用户参考图、生成图、转换后的动画、导入的宠物文件和输出目录都属于本机用户数据。
- 不允许把真实用户素材路径写入源码、文档示例、测试快照或提交记录。
- 不允许提交生成目录、用户导入宠物、运行缓存、API Key、token、`.env*` 或本机配置。
- 如果未来接入第三方生成服务，必须明确列出会上传什么内容，并默认要求用户确认。
- 当前音乐功能规则继续保留：音乐文件路径、播放列表、存储目录不参与宠物素材生成，也不能被上传。

## UI 要求

后续如果新增 AI 宠物生成 UI，需要至少包含：

- 参考图选择。
- 宠物名称输入。
- 风格选择：自动、像素、贴纸、毛绒、黏土、扁平矢量、3D 玩具等。
- 生成输出目录选择。
- 生成进度状态：准备中、生成基础形象、生成动作、转换中、完成、失败。
- 结果预览：PetDrawer 已支持的多状态动画，至少包含待机状态。
- 错误提示：缺少参考图、生成失败、转换失败、导入失败。
- 隐私提示：素材和路径只保存在本机。

所有控件必须同时检查：

- 默认主题。
- `animal-island` 主题。
- 小窗口宽度下的文字换行和按钮布局。
- 错误、空状态、加载状态和禁用状态。

## 转换器需求

后续建议新增一个独立转换流程，职责是把 `hatch-pet` 输出转换成 PetDrawer 可直接导入的素材。

输入：

- `hatch-pet` 运行目录。
- 当前固定标准状态映射；自定义映射留到后续。

输出：

- `idle.gif` 或 `idle.webp`
- `hover.gif` 或 `hover.webp`
- `click.gif` 或 `click.webp`
- `dragging.gif` 或 `dragging.webp`
- 可选的 `draggingLeft`、`draggingRight`、`waving`、`jumping`、`waiting`、`running`、`review`、`failed` 动画文件
- PetDrawer 兼容 `pet.json`
- 可选预览图或 contact sheet
- 转换摘要：图集版本、尺寸、结构检查、QA 证据、警告和输出状态清单

验收：

- 每个输出动画能被 PetDrawer 当前导入逻辑读取。
- `idle` 必须存在。
- 缺少可选状态时可以回退到 `idle`，但要在结果摘要中说明。
- 输出文件名不能覆盖已有同名文件，必要时自动追加后缀。
- 输出目录不应默认指向源码目录。
- v2 缺少强制 QA 文件时允许转换预览，但必须明确显示“视觉质量未验证”，不能显示为完整通过。
- 旧 8 x 9 图集允许兼容转换，但必须说明缺少 16 个注视方向。

## 验收标准

### 文档阶段

- [x] 明确当前 PetDrawer 和 `hatch-pet` 的状态差异。
- [x] 明确隐私边界。
- [x] 明确后续阶段路线图。
- [x] 明确默认主题和 `animal-island` 主题检查要求。

### MVP 实现阶段

- [ ] 能从一张参考图或文字描述生成宠物素材。
- [x] 能把生成结果转换成 PetDrawer 当前多状态格式。
- [x] 能通过现有导入入口导入并使用。
- [x] 不把用户图片、生成素材、本机路径、API Key 或 token 写入仓库。
- [x] 转换失败和导入失败都有清晰错误提示。

### UI 实现阶段

- [x] 默认主题的转换器入口和状态样式已实现并通过结构测试。
- [x] `animal-island` 主题的转换器入口和状态样式已实现并通过结构测试。
- [x] 转换卡片、按钮组和警告列表使用响应式布局；真实素材的人工视觉验收仍需用户在目标显示器上复核。
- [x] 转换中和导入中状态禁止重复触发转换或导入操作。

## 建议下一步

1. 先实现一个离线转换器，把 `hatch-pet` 的 atlas 或 row strips 转成 PetDrawer 的多状态动画文件。
2. 用一组临时测试素材验证转换器，不提交用户素材。
3. 再把转换器接入现有“导入宠物”流程，减少手动选择动画的步骤。
4. 最后再考虑应用内 AI 生成入口、AI 聊天状态联动和音乐状态联动。

## 本次实施计划：银月宠物素材和多状态动画

当前对照状态：PetDrawer 多状态结构、拖动方向播放、导入编辑预览和旧包回退已完成；银月素材本身、`hatch-pet` 输出转换器和应用内生成入口仍未完成。

本次目标是根据用户提供的银月参考图生成一套可用于 PetDrawer 的宠物形象和动画，并根据生成结果调整宠物动画状态数量。参考图的真实本机路径不写入本文档和源码。

用户后续明确要求使用 `hatch-pet` 重新制作，因此最终验收以 `hatch-pet` 的 Codex pet atlas 流程为准：先生成基础主形象，再生成 `idle`、`running-right`、`running-left`、`waving`、`jumping`、`failed`、`waiting`、`running`、`review` 九行动作素材，之后再转换为 PetDrawer 可导入的状态动画包。

### 形象目标

- 保留参考图的核心识别点：银白长发、大兽耳、白色蓬松尾巴、蓝灰色大眼睛、白银配色服饰、轻盈精致的 Q 版比例。
- 输出必须适合桌面宠物小尺寸展示：轮廓清晰、脸部可读、尾巴和耳朵不被裁切、服饰细节不过度碎片化。
- 动画素材背景应可转换为透明背景，不能包含文字、水印、UI、格子线或不可清理的复杂背景。

### 本次优先产物

- 基础主形象图。
- 多状态动作素材。
- PetDrawer 可读取的本机宠物包。
- 动画预览图或 GIF。
- 如需要，调整 PetDrawer 的动画状态结构，让更多生成动作能被保存、导入、预览和播放。

### 动画状态策略

本次已允许从旧版基础状态扩展到多状态。PetDrawer 状态优先从 `hatch-pet` 状态映射而来：

- `idle`：待机。
- `hover`：选中或鼠标悬停。
- `click`：点击反馈。
- `dragging`：拖动。
- `waiting`：等待用户输入或批准。
- `running`：AI 正在处理。
- `review`：AI 回复完成或检查结果。
- `failed`：AI 请求失败、取消或阻塞。
- `waving`：打招呼或轻互动。
- `jumping`：跳跃或活泼反馈。

如果生成素材质量不足以支撑全部状态，允许先保留质量稳定的状态，并让缺失状态回退到 `idle`。

状态映射：

- `hatch-pet idle` -> PetDrawer `idle`
- `hatch-pet review` 或 `waving` -> PetDrawer `hover`
- `hatch-pet jumping` 或 `waving` -> PetDrawer `click`
- `hatch-pet running-right` -> PetDrawer `dragging`
- `hatch-pet waving` -> PetDrawer `waving`
- `hatch-pet jumping` -> PetDrawer `jumping`
- `hatch-pet waiting` -> PetDrawer `waiting`
- `hatch-pet running` -> PetDrawer `running`
- `hatch-pet review` -> PetDrawer `review`
- `hatch-pet failed` -> PetDrawer `failed`

### 状态语义验收

本次银月动画必须参考 Codex pet 的状态语义，不允许所有状态只用同一种缩放、摇摆或位移做弱区分。使用 `hatch-pet` 重新制作时，优先检查 9 行 row strip 的动作语义，再检查转换后的 PetDrawer 动画。

- `idle`：只能是安静呼吸、眨眼、轻微头身起伏，不能表现成互动或工作。
- `hover`：表现“被选中/注意到用户”，可比待机更精神，但不能等同点击或跳跃。
- `click`：表现明确点击反馈，允许短促压缩、回弹或轻快反应。
- `dragging`：表现被拖动时的方向性或悬挂感，和普通待机不同。
- `waving`：通过手、袖子或身体侧向摆动表现打招呼，不使用漂浮波纹和文字。
- `jumping`：通过垂直高度变化表现跳跃，不使用地面阴影、尘土或独立特效。
- `waiting`：表现等待用户输入、批准或帮助，姿态应更期待、询问或停顿。
- `running`：表现正在处理、思考或努力工作，不能做成字面跑步。
- `review`：表现完成后检查、审视或确认结果，区别于等待和运行中。
- `failed`：表现失败、阻塞或取消后的低落、变暗或垂头反应，不能使用红叉、文字或漂浮符号。

### 本次追加要求：拖动方向和播放速度

用户进一步要求拖动时根据移动方向播放不同跑步方向，并提高帧率到 24fps 同时放慢动作速度。本次按以下规则验收：

- PetDrawer 保留旧 `dragging` 状态，新增 `draggingLeft` 和 `draggingRight` 两个可选动画状态。
- 运行时拖动窗口时根据桌宠窗口横向位移切换 `draggingLeft` 或 `draggingRight`，横向位移不足时保持最近一次拖动方向。
- 旧宠物包如果没有左右拖动动画，继续回退到 `dragging`，再回退到 `idle`，避免破坏兼容性。
- 银月重制包额外输出 `draggingLeft.webp` 和 `draggingRight.webp`，分别来自 `hatch-pet running-left` 和 `running-right`。
- 银月重制包按 24fps 导出动画文件，但通过重复帧延长循环时长，让动作更慢、更顺滑。
- 新增字段必须能被导入宠物包、编辑宠物动画、预览列表和当前宠物运行时读取。

### 实施顺序

1. 准备 `hatch-pet` 运行目录和生成任务清单。
2. 基于参考图生成银月基础主形象。
3. 生成可用动作行或动作帧。
4. 转换成 PetDrawer 可导入和可播放的动画文件。
5. 扩展 PetDrawer 动画状态类型、导入数据结构和预览 UI。
6. 检查默认主题和 `animal-island` 主题下的新增状态标签、导入表单和预览表现。
7. 执行隐私检查，确认没有提交用户参考图、生成素材、本机路径、密钥或本机配置。
