# 沉浸模式 WebGL 星辰可视化需求计划

本文档记录音乐沉浸模式新增 WebGL 星辰可视化的需求和实现边界。该功能只使用当前运行态的频谱摘要和界面主题颜色，不保存用户歌曲路径、播放 URL、歌词内容、音频数据、音频指纹、平台 Cookie、token 或本机运行历史。

## 背景

- 当前“星辰”频谱样式由 2D Canvas 绘制，增强光晕后会受到每帧渐变、阴影和绘制次数影响，复杂效果容易卡顿。
- Mineradio 的主视觉星河采用 GPU 粒子思路：预生成粒子点位，每帧只更新音乐能量和主题颜色，由 shader 完成运动、闪烁和光晕。
- 本项目可以参考这种工程方向，但不能复制 Mineradio 的源码、shader、素材、Logo、命名或 GPL 实现。

## 目标

1. 新增 Three.js `Points + ShaderMaterial` 星辰可视化层，用 GPU 绘制星点、星云丝带和柔光层。
2. “星辰”频谱样式优先使用 WebGL 渲染；WebGL 初始化失败、上下文丢失或用户开启降低动态时仍有稳定 Canvas 兜底。
3. WebGL 星辰只接收运行态摘要数据：
   - `bass`
   - `mid`
   - `treble`
   - `beat`
   - `volume`
   - `playing`
   - `intensity`
   - `reducedMotion`
   - 当前沉浸主题调色板
4. 每帧只更新 shader uniforms，不在动画循环中创建 geometry、material、texture、数组或渐变。
5. 星辰 WebGL 层和现有 Canvas 背景、线条、波纹协作：WebGL 负责星辰频谱层，Canvas 继续负责背景、网格、线条和波纹。
6. 默认主题、`animal-island` 主题和新增沉浸主题均使用当前主题调色板，不出现固定单色或主题割裂。
7. WebGL 资源在组件卸载、样式切换和窗口关闭时释放，避免显存泄漏。

## 非目标

- 不改音乐播放链路、在线平台登录、歌词读取、歌单逻辑或代理逻辑。
- 不读取、上传或保存原始音频、歌曲文件路径、播放链接、歌词内容、平台凭据或本机资源路径。
- 不复制 Mineradio 的 GPL shader、粒子坐标、视觉命名、预设结构或素材。
- 不把全部沉浸可视化迁移到 Three.js；本次只处理星辰频谱层。

## 方案

### 依赖

- 新增 `three` 作为前端依赖，新增 `@types/three` 作为开发类型依赖。
- 组件运行时通过动态 `import('three')` 按需加载 WebGL 代码，避免普通桌宠和音乐窗口首屏加载 Three.js。
- 通过 TypeScript 类型导入 Three.js，不把 vendor 文件复制进仓库。

### 组件

- 新增 `MusicWebglStarfield.vue`：
  - 独立创建 WebGL canvas。
  - 初始化 `WebGLRenderer`、`Scene`、`OrthographicCamera`、两层 `Points`。
  - 主粒子层绘制清晰星点。
  - 柔光粒子层使用更大点尺寸和 AdditiveBlending 绘制光晕。
  - 通过 `webgl-unavailable` 事件通知父组件回退 Canvas 星辰。

### 粒子数据

- 初始化阶段生成固定数量粒子：
  - 普通模式：约 1100 到 1600 个点。
  - 降低动态：约 520 到 760 个点。
- 每个粒子保留：
  - 基础位置
  - 随机种子
  - 轨道层
  - 频段映射
  - 基础大小
- 粒子属性进入 `BufferGeometry`，动画循环中不重新分配。

### Shader

- vertex shader 根据时间、音乐能量和随机种子计算：
  - 轨道漂移
  - 星点闪烁
  - 低频外扩
  - 节拍脉冲
  - 降低动态时的低速和低振幅
- fragment shader 使用一次性生成的圆形点纹理，让星点边缘自然衰减。
- 光晕层复用同一 geometry，以更大点尺寸和更低 alpha 叠加。

### 回退

- 父组件在 `spectrumStyle === 'particles'` 时尝试显示 WebGL 星辰层。
- WebGL 可用时，现有 Canvas 频谱层切到 `none`，避免重复绘制星辰。
- WebGL 不可用时，继续使用当前 Canvas “星辰”绘制函数。

## 主题检查

1. 默认主题：星点、光晕和背景可读，不遮挡歌词和控制栏。
2. `animal-island` 主题：星点颜色柔和不过曝，按钮继承现有主题样式。
3. 星河、霓虹、电影暗场、暖色舞台和深夜睡眠主题：WebGL 星辰使用主题色，不使用固定外部调色板。
4. 降低动态：减少粒子数量、旋转速度、闪烁强度和光晕强度。

## 验收标准

- [x] “星辰”频谱样式可使用 WebGL 星辰层渲染。
- [x] WebGL 不可用或上下文丢失时可以回退 Canvas 星辰，不出现空白。
- [x] 切换到其他频谱样式时 WebGL 层隐藏并释放高成本动画。
- [x] 播放、暂停、无频谱数据时都有稳定表现。
- [x] 默认主题、`animal-island` 主题和新增沉浸主题下可读、不重叠。
- [x] `npm run build` 通过。
- [x] `npm run tauri:build` 通过并启动 release 版。
- [x] 隐私检查清单通过。
