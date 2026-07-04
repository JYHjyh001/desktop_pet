# 地面 DJ Shader 高度驱动改造需求计划

本文档记录参考 `yin-yizhen/sonic-topography` 的 GPU 顶点高度驱动方式，对当前沉浸音乐“地面 DJ”柱体地形进行低风险改造。本次只调整前端 WebGL 渲染路径、运行态音乐能量摘要和本机视觉参数，不保存用户歌曲路径、播放 URL、歌词文本、平台 Cookie、token、音频数据或本机运行历史。

## 背景

- 当前地面 DJ 已经具备圆形柱阵、频谱通量触发点、水波扩散、主题色和视距/高度/响应/密度/波峰/触发调参。
- 当前柱体高度在 JavaScript 帧循环中逐个计算，再通过 `setMatrixAt` 更新实例矩阵；密度提高后 CPU 侧遍历和矩阵更新压力较明显。
- Sonic Topography 的可迁移原则是：CPU 只传音频 uniform、触发点和少量 per-instance 属性，柱体高度与波纹影响尽量交给 vertex shader 计算。

## 目标

1. 保留当前地面 DJ 的圆形/环形布局、主题调色、触发点类型和舞台参数。
2. 为地形实例增加静态属性：频段、环位、角度、螺旋、随机种子、边缘雾化等。
3. 每帧只更新音乐能量、舞台参数、主题色和触发点 uniform，不再逐帧为每根柱体计算高度和颜色。
4. 在地形 vertex shader 中计算：
   - 中心低频抬升。
   - 低频块状起伏。
   - 低中频慢波。
   - 中频径向流。
   - 高频外圈尖峰。
   - pulse/snare/spark 触发点扩散波前。
5. 在 fragment shader 中用 shader 计算得到的高度、波纹和高频尖峰驱动顶面高光、侧面亮度、主题色混合和深度雾化。
6. 默认主题和 `animal-island` 主题下继续保持远景自然融入背景，柱体不透明且不闪白。

## 非目标

- 不复制 Sonic Topography 的源码、shader、素材、Logo、命名或受限实现。
- 不修改音乐播放、歌词解析、平台账号和本地曲库保存链路。
- 不把星河漫游改成柱体地形；本次只影响“地面 DJ”预设。
- 不改变用户歌曲路径、播放 URL、歌词文本、Cookie、token 或音频数据的持久化策略。

## 实施步骤

1. 新增地形实例静态属性和 shader uniform 类型。
2. 创建地形层时只初始化实例位置、尺寸和静态属性。
3. 将活动触发点打包为固定长度 uniform 数组。
4. 将柱体高度、波纹影响、颜色混合和雾化逻辑迁入地形 shader。
5. 保留 `frequencyData` 正向通量触发器和能量回退触发器。
6. 清理不再需要的逐柱高度/颜色 CPU 缓冲。
7. 更新完成/未完成功能记录。
8. 执行隐私检查、构建检查，并按桌宠重启规则运行 release 版。

## 验收标准

- [x] 地面 DJ 仍显示圆形/环形柱体声场，默认视野不出现方形硬边。
- [x] 地面 DJ 柱体高度由 shader 随 bass/mid/treble/beat/volume 和触发点变化。
- [x] 高度、响应、波峰、触发和密度调节仍可见生效；密度只在重建柱阵时生效。
- [x] pulse/snare/spark 触发点仍能扩散波前，强低频扩散更明显。
- [x] 星河漫游不显示地面 DJ 柱体，既有星河参数不受影响。
- [x] 默认主题和 `animal-island` 主题下柱体颜色、远景雾化和控制项表现正常。
- [x] `npm run build` 通过。
- [x] `npm run tauri:build` 通过并启动 `src-tauri\target\release\pet_drawer.exe`。
- [x] 隐私检查未发现真实密钥、本机歌曲路径、播放 URL、歌词文本或平台凭据进入源码/文档。

## 验证记录

- 已执行 `npm run build`，`vue-tsc --noEmit` 和 `vite build` 通过。
- 已用系统 Chrome + Playwright 临时 mock Tauri `music` 窗口，分别检查默认主题和 `animal-island` 主题：均能进入 `music-immersive is-stage-dj`，WebGL canvas 存在，地面 DJ 圆形柱阵/网格可见，右侧韵律面板和底部控制栏未出现明显错位。
- WebGL canvas 直接读取像素时因 Three.js 默认未启用 `preserveDrawingBuffer` 返回透明；视觉校验以页面截图为准。
- 已执行 `npm run tauri:build`，Tauri release 构建通过，并已启动 `src-tauri\target\release\pet_drawer.exe`。
- 已按 `docs/RELEASE_PRIVACY_CHECKLIST.md` 执行状态、忽略规则和疑似密钥关键词检查；本次新增内容没有写入真实 API Key、token、用户歌曲路径、播放 URL、歌词文本、平台 Cookie、音频数据或本机运行历史。
