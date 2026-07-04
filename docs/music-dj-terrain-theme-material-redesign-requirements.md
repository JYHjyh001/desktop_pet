# 地面 DJ 主题材质与柱体样式重设计需求计划

本文档记录参考 `yin-yizhen/sonic-topography` 的地形主题 token、音色驱动材质和顶面/侧面/边缘分层着色思路，对当前沉浸音乐“地面 DJ”柱体样式进行视觉重设计。本次只调整前端 WebGL 运行态显示、主题色映射和 shader 材质参数，不保存用户歌曲路径、播放 URL、歌词文本、平台 Cookie、token、音频数据或本机运行历史。

## 背景

- 当前地面 DJ 已经把柱体高度、波纹影响和高光迁入 shader，但主题仍主要复用通用 `visualPalette` 的低频、中频、高频、脉冲和阴影色。
- 这种方式能表现柱体起伏，但缺少独立的 DJ 材质身份：柱身、顶面、边缘和远景混色层级不够清晰，`animal-island` 主题下容易偏黄褐和发闷。
- Sonic Topography 的可迁移原则是：主题不只是换色，而是拆成背景底色、柱体暗底、冷色核心、冷色边缘、暖色核心、暖色边缘、波纹色和发光强度；不同频段再分别控制高度、色温、闪烁和边缘微光。

## 目标

1. 为地面 DJ 增加专用材质 palette，不再直接用通用 `visualPalette` 控制柱体外观。
2. 默认主题和 `animal-island` 主题都获得明确的 DJ 材质身份：
   - 默认主题：深蓝黑柱身、青蓝顶光、琥珀节拍波、少量白色顶沿闪烁。
   - `animal-island`：深橄榄/木色柱身、薄荷顶光、奶油色顶沿、柔和琥珀波纹，避免发灰和脏黄。
3. shader 中区分柱身、顶面、边缘、波纹、微闪和远景雾化。
4. 从现有 bass/mid/treble/beat/volume 中派生 `presence`、`brilliance`、`air`、`warmth`、`brightness`、`sharpness` 等音色材质量，用于局部闪光、边缘微闪、空气颗粒和暖冷色温切换。
5. 保持现有柱阵布局、触发点、舞台参数和星河漫游逻辑不变。
6. 默认主题和 `animal-island` 主题都要做视觉检查。

## 非目标

- 不复制 Sonic Topography 的源码、shader、素材、Logo、命名或受限实现。
- 不新增地面 EQ 设置界面；本次只做内部材质映射和 shader 视觉升级。
- 不修改音乐播放、歌词解析、平台账号、本地曲库保存和在线播放代理链路。
- 不把用户歌曲路径、播放 URL、歌词文本、Cookie、token 或音频数据写入源码或文档示例。

## 实施步骤

1. 新增地面 DJ 专用材质 palette 类型和主题映射函数。
2. 扩展地形 shader uniforms，加入材质底色、冷暖核心色、波纹色、顶沿色、闪光色和发光强度。
3. 在 `updateTerrainLayer` 中从现有音乐能量派生音色材质量，并更新材质 uniforms。
4. 重写 terrain fragment shader 的柱身、顶面、边缘、波纹、微闪和雾化混色逻辑。
5. 保持 vertex shader 的高度和波纹驱动不变，只补充材质所需的距离、随机、边缘等 varying。
6. 更新完成/未完成功能记录。
7. 执行隐私检查、构建检查、默认主题和 `animal-island` 主题视觉检查，并按桌宠重启规则运行 release 版。

## 验收标准

- [x] 地面 DJ 柱体有明确的暗色柱身、明亮顶面和细边缘高光，整体不再是一层简单混色。
- [x] 默认主题下柱体呈现深蓝黑底、青蓝顶光和暖色节拍波，远景自然融入背景。
- [x] `animal-island` 主题下柱体呈现木色/橄榄暗底、薄荷顶光和奶油顶沿，避免脏黄、灰蒙和过曝。
- [x] 高频派生的 `presence`、`brilliance`、`air` 会带来轻量局部闪光、边缘微闪和空气颗粒，但不闪白刺眼。
- [x] 星河漫游不受地面 DJ 材质改造影响。
- [x] `npm run build` 通过。
- [x] `npm run tauri:build` 通过并启动 `src-tauri\target\release\pet_drawer.exe`。
- [x] 隐私检查未发现真实密钥、本机歌曲路径、播放 URL、歌词文本或平台凭据进入源码/文档。

## 验证记录

- 已执行 `npm run build`，`vue-tsc --noEmit` 和 `vite build` 通过。
- 已用 `dist` 静态服务 + 系统 Chrome + Playwright 临时 mock Tauri `music` 窗口，分别检查默认主题和 `animal-island` 主题：均能进入 `music-immersive is-stage-dj`，WebGL canvas 存在，地面 DJ 柱阵可见，右侧韵律面板和底部控制栏未出现明显错位。
- 测试 mock 未完整提供抽屉窗口配置和宠物皮肤列表，因此控制台出现抽屉窗口初始化错误；音乐窗口和地面 DJ 检查结果正常。
- 已按桌宠修改后重启规则执行 `npm run tauri:build`，前端 production build、Tauri release 编译、msi/nsis 打包均通过。
- 已启动 `src-tauri\target\release\pet_drawer.exe`，进程路径确认为当前项目 release 版桌宠程序。
- 已按 `docs/RELEASE_PRIVACY_CHECKLIST.md` 执行 `git status --short`、`.gitignore` 和疑似密钥关键词检查；本次新增内容没有写入真实 API Key、token、用户歌曲路径、播放 URL、歌词文本、平台 Cookie、音频数据或本机运行历史。
- 根据实际观感反馈继续调快从舞台中心扩散的大片地面突起波，其他随机触发点、中频和高频波纹速度保持原逻辑。
- 中心低频波纹调速后已再次执行 `npm run build`、默认主题和 `animal-island` 主题生产构建截图检查、`npm run tauri:build`，并启动当前项目 release 版桌宠程序。
- 按最新反馈参考 Sonic Topography 的 Kick 响应分层方式，新增独立中心波 uniform，只让固定中心波纹使用 `24.0` 试验基础速度；随机低频 `pulse` 保持原始 `3.05` 速度、`0.34` 波面宽度和 `5.15` 拖尾参数，中高频 `snare`/`spark` 波纹速度不变。
- 参考 Sonic Topography 的 Kick envelope 表现，为地面 DJ 新增独立 `terrainKickEnvelope`：低频中心波触发时给中心和内圈短促冲击高度，并增加中心高光和暖色响应；该冲击不写回整体 `bass`/`subBass`，避免整片地形被一起抬高。
- 中心 Kick 冲击抬升实现后，已再次执行 `npm run build`、默认主题和 `animal-island` 主题生产构建截图检查、`npm run tauri:build`，并启动当前项目 release 版桌宠程序。
- 中心波调整到 `24.0` 后，已再次执行 `npm run build`、默认主题和 `animal-island` 主题生产构建检查、`npm run tauri:build`，并启动当前项目 release 版桌宠程序。
- 根据实际观感反馈，将中心波与随机 `pulse` 分离：中心波固定从中心扩散并使用 `24.0` 试验速度，随机 `pulse` 触发点保持原始 `3.05` 速度和原波面参数；Kick 抬升只由中心波触发，避免随机低频触发点被中心波纹调速一起加快。
- 中心/随机低频波纹拆分后，已再次执行 `npm run build`、默认主题和 `animal-island` 主题生产构建检查、`npm run tauri:build`，并启动当前项目 release 版桌宠程序。
- 已按最新纠偏确认：频谱通量和 fallback 低频触发都保留原随机 `pulse` 生成、强度、`3.05` 速度、`0.34` 波面宽度和 `5.15` 拖尾；仅通过独立 `uCenterPulseData` 叠加固定中心波，并只让它使用 `24.0` 试验速度和中心 Kick 抬升。
- 参考 Sonic Topography 的责任分离方式继续修缮后，中心大片地面突起波改为独立 `uCenterPulseData` uniform，不再进入普通 `uRipple*` 环形队列；随机 `pulse`、`snare`、`spark` 的队列数量、位置、速度和衰减保持原逻辑。已再次执行 `npm run build`，并检查默认主题和 `animal-island` 主题生产构建下的地面 DJ 舞台。
