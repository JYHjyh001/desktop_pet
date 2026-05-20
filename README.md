# PetDrawer 桌面宠物软件抽屉

PetDrawer 是一个基于 Tauri v2 + Vue 3 + TypeScript + Rust 的桌面宠物软件启动器。

## 已实现的第一版能力

1. 启动后创建 `pet` 宠物窗口：透明、无边框、置顶、不显示在任务栏。
2. 宠物支持点击、拖动和右键菜单；右键菜单使用独立 `pet-menu` 窗口，避免被宠物窗口裁剪。
3. 点击宠物显示或隐藏 `drawer` 软件抽屉窗口。
4. 抽屉窗口支持添加、编辑、删除、搜索、分类筛选本地 exe 软件。
5. 支持规范化宠物形象库，每个宠物可包含待机、选中、点击、拖动等多状态动画。
6. 支持为每个软件设置自定义图标；选择 exe 路径后会自动尝试提取软件图标，也可以手动选择图标。
7. 软件列表保存到 Tauri 应用数据目录的 `apps.json`。
8. 宠物位置、自定义宠物形象、分类、快捷搜索、标签显示模式和窗口置顶设置保存到应用数据目录的 `config.json`。
9. 启动软件时，前端只传 `app_id`，Rust 后端读取 `apps.json` 后启动对应程序。
10. 系统托盘提供显示宠物、隐藏宠物、打开抽屉和退出程序。
11. 抽屉设置中可以编辑分类选项、快捷搜索按钮、软件显示方式，以及分别控制宠物和抽屉是否置顶。
12. 最近 7 天内启动 2 次及以上的软件会自动加入“常用”，自动加入的软件超过 7 天未启动会自动移出。

## 普通用户下载和使用步骤

普通用户只需要下载已经打包好的安装包或 exe 文件，不需要安装 Node.js、npm、Rust、Cargo 或 Microsoft C++ Build Tools。

### 下载程序

1. 从发布者提供的下载链接中下载最新版 PetDrawer。
2. 优先下载安装包文件，文件名通常类似：

```txt
PetDrawer_0.1.0_x64-setup.exe
```

3. 如果发布者提供的是免安装版本，文件名通常类似：

```txt
pet_drawer.exe
```

4. 不要下载源码压缩包来直接运行。源码包通常包含 `src`、`src-tauri`、`package.json` 等文件，只适合开发者使用。

### 安装版本使用步骤

1. 双击 `PetDrawer_0.1.0_x64-setup.exe`。
2. 如果 Windows 弹出安全提示，确认文件来自可信发布者后，选择“更多信息”，再选择“仍要运行”。
3. 按安装向导完成安装。
4. 安装完成后，从桌面快捷方式、开始菜单或安装目录启动 PetDrawer。
5. 启动后，桌面上会出现一个宠物窗口。
6. 点击宠物可以打开或隐藏软件抽屉。
7. 右键点击宠物可以打开宠物菜单。
8. 通过系统托盘图标可以显示宠物、隐藏宠物、打开抽屉或退出程序。

### 免安装版本使用步骤

1. 将 `pet_drawer.exe` 放到一个固定目录，例如：

```txt
D:\Apps\PetDrawer\
```

2. 双击 `pet_drawer.exe` 启动程序。
3. 不建议把 exe 放在临时目录、下载缓存目录或会被自动清理的目录中。
4. 如果需要创建桌面快捷方式，右键 `pet_drawer.exe`，选择“发送到” -> “桌面快捷方式”。

### 添加自己的软件

1. 启动 PetDrawer 后，点击桌面宠物打开软件抽屉。
2. 点击“添加软件”。
3. 在“软件路径”中选择本机已有的 `.exe` 程序。
4. 填写软件名称、分类和标签。
5. 保存后，软件会显示在抽屉中。
6. 后续点击软件卡片即可快速启动该软件。

### 更换宠物形象

1. 打开软件抽屉。
2. 点击左上角“当前宠物形象”区域中的“更换”。
3. 选择已有宠物，或导入自己的宠物图片。
4. 导入宠物时，至少需要选择一个待机动画图片。
5. 支持的图片格式包括 `png`、`jpg`、`jpeg`、`webp`、`gif`、`ico`。

### 数据保存位置

PetDrawer 会把软件列表、设置、图标和导入的宠物保存到系统应用数据目录中。卸载或移动程序前，如果需要保留数据，可以先备份 PetDrawer 的应用数据目录。

保存的数据包括：

```txt
apps.json      软件列表
config.json    窗口位置、当前宠物、分类、快捷搜索等设置
icons/         自定义软件图标
pets/          导入的宠物形象
```

### 常见问题

1. 如果双击后没有反应，先检查程序是否被杀毒软件拦截。
2. 如果系统提示缺少 WebView2 Runtime，请安装 Microsoft Edge WebView2 Runtime 后再启动。Windows 10/11 通常已经自带。
3. 如果 Windows 提示“已保护你的电脑”，确认文件来源可信后，点击“更多信息” -> “仍要运行”。
4. 如果添加的软件无法启动，检查该软件路径是否仍然存在，或重新编辑软件路径。
5. 如果宠物或抽屉位置异常，可以退出程序后重新启动。
6. 如果要彻底退出程序，请使用系统托盘菜单中的“退出”，不要只关闭抽屉窗口。

## 开发运行步骤

### 前置环境

Windows 上运行 Tauri 需要先准备：

1. Node.js 和 npm。
2. Rust 工具链，安装后命令行中必须能执行 `cargo --version`。
3. Microsoft C++ Build Tools，安装时选择“使用 C++ 的桌面开发”工作负载。
4. Microsoft Edge WebView2 Runtime，Windows 10/11 通常已内置。

如果运行 `npm run tauri:dev` 时出现：

```txt
failed to run 'cargo metadata' ... program not found
```

说明 Cargo 没有安装，或 `%USERPROFILE%\.cargo\bin` 没有加入 `PATH`。

处理步骤：

1. 打开 https://rustup.rs/ 下载并运行 `rustup-init.exe`。
2. 安装选项选择默认配置，确保使用 MSVC toolchain。
3. 安装完成后关闭当前 PowerShell，重新打开一个新的 PowerShell。
4. 执行：

```bash
cargo --version
rustc --version
```

5. 如果仍然找不到 `cargo`，把下面路径加入系统环境变量 `Path`：

```txt
%USERPROFILE%\.cargo\bin
```

### 启动项目

1. 安装前端依赖：

```bash
npm install
```

2. 启动 Tauri 开发环境：

```bash
npm run tauri:dev
```

项目会在启动前自动检查并创建 `src-tauri/icons/icon.ico`。如果你看到 `icons/icon.ico not found`，可以手动执行：

```bash
npm run create-icon
npm run tauri:dev
```

3. 构建前端：

```bash
npm run build
```

4. 打包桌面应用：

```bash
npm run tauri:build
```

检查更新功能会读取 GitHub Releases 最新正式版本：

```txt
https://github.com/JYHjyh001/desktop_pet/releases/latest
```

每次发布新版时，建议按下面步骤处理：

1. 同步修改版本号：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`。
2. 执行 `npm run tauri:build`。
3. 在 GitHub 仓库创建新的 Release。
4. Release tag 使用 `v版本号` 格式，例如 `v0.1.1`。
5. 把打包生成的 `.exe` 文件上传到 Release 附件中。
6. 发布正式 Release，不要勾选 prerelease。

程序会请求 GitHub 最新正式 Release，比较本地版本和 Release tag，并优先选择 `.exe` 附件作为下载入口。

## 目录结构

```txt
src/
  assets/pets/default/ 内置默认宠物动画
  windows/
    PetWindow.vue       宠物窗口
    DrawerWindow.vue    软件抽屉窗口
  components/           抽屉和宠物 UI 组件
  stores/appStore.ts    软件数据状态管理
  types/app.ts          前端类型
  utils/format.ts       文本格式化工具
src-tauri/
  src/app_data.rs       JSON 数据读写
  src/commands.rs       Tauri 命令
  src/launcher.rs       本地软件启动
  src/tray.rs           系统托盘
  src/windowing.rs      宠物和抽屉窗口控制
```

## 数据保存位置

程序会使用 Tauri 的应用数据目录，首次启动时自动创建：

```txt
PetDrawer/
  apps.json
  config.json
  icons/
  pets/
    skins/
      skin_xxx/
        pet.json
        idle.png
        hover.png
        click.png
        dragging.png
```

## 自定义宠物形象

项目内置默认宠物位于：

```txt
src/assets/pets/default/
  pet.json
  preview.svg    凯特展示图
  idle.svg       待机：轻微呼吸并眨眼
  hover.svg      选中：惊讶睁眼和弹起
  click.svg      点击：微笑并冒爱心
  dragging.svg   拖动：抓住小把手悬挂摆动
```

1. 更换宠物：打开软件抽屉，左上角“当前宠物形象”区域会显示当前宠物，点击“更换”。
2. 搜索已有宠物：更换窗口会先读取应用数据目录下的 `pets/skins/`，并展示每个宠物的预览。
3. 查看宠物信息：点击宠物卡片后，右侧会显示宠物名称、来源和待机/选中/点击/拖动动画配置。
4. 选择宠物：点击预览卡片即可切换，切换结果会写入 `config.json`，重启后仍会保留。
5. 导入宠物：在更换窗口下方“导入宠物”区域填写名称，并选择动画图片。
6. 动画规范：`待机动画` 必填，`选中动画`、`点击动画`、`拖动动画` 可选；未设置的状态会自动使用待机动画。
7. 支持格式：`png`、`jpg`、`jpeg`、`webp`、`gif`、`ico`。
8. 导入后的文件会复制到 `pets/skins/<skin_id>/`，并生成 `pet.json`。

`pet.json` 示例：

```json
{
  "id": "skin_1790000000000",
  "name": "小猫助手",
  "animations": {
    "idle": "pets/skins/skin_1790000000000/idle.png",
    "hover": "pets/skins/skin_1790000000000/hover.gif",
    "click": "pets/skins/skin_1790000000000/click.gif",
    "dragging": "pets/skins/skin_1790000000000/dragging.gif"
  },
  "createdAt": "1790000000"
}
```

## 自定义软件图标

1. 添加或编辑软件时，先在“软件路径”区域点击“选择”并选中 exe 文件。
2. 程序会自动尝试从 exe 提取关联图标，并复制到应用数据目录。
3. 如果自动提取失败，仍可在“软件图标”区域点击“选择图标”手动选择图片。
4. 选择后的图片会复制到应用数据目录，后续不依赖原始图片路径。

## 抽屉设置

1. 打开抽屉后，点击右上角“设置”。
2. 在“分类选项”中新增或删除分类；`全部`、`常用`、`其他` 为核心分类，会自动保留。
3. 在“快捷搜索”中新增或删除搜索按钮；这些按钮会显示在搜索框和“添加软件”按钮中间。
4. 在“软件显示方式”中选择“缩略显示”或“详细显示”；缩略显示保留软件图标、软件名称和标签，点击卡片即可启动软件。
5. 在“窗口置顶”中分别控制宠物窗口和抽屉窗口是否始终置顶。
6. 在“软件更新”中查看当前版本，并检查发布者是否配置了新版下载入口。

## 自动常用规则

1. 软件每次启动成功后，会记录一次启动时间。
2. 最近 7 天内启动 2 次及以上的软件，会自动进入“常用”分类。
3. 自动进入“常用”的软件，如果超过 7 天没有再次启动，会自动从“常用”分类移出。
4. 手动勾选“设为常用”的软件不会被自动移出。
