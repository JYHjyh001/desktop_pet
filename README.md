# PetDrawer 桌面宠物快捷入口抽屉

PetDrawer 是一个基于 Tauri v2 + Vue 3 + TypeScript + Rust 的桌面宠物快捷入口启动器。

## 已实现的第一版能力

1. 启动后创建 `pet` 宠物窗口：透明、无边框、置顶、不显示在任务栏。
2. 宠物支持点击、拖动和右键菜单；右键菜单使用独立 `pet-menu` 窗口，避免被宠物窗口裁剪。
3. 点击宠物显示或隐藏 `drawer` 软件抽屉窗口。
4. 右键菜单中可以选择“对话”，打开独立 `pet-chat` 窗口和宠物聊天。
5. 抽屉窗口支持添加、编辑、删除、搜索，并按分类和入口类型筛选本地软件、文件夹和网站。
6. 支持规范化宠物形象库，每个宠物可包含待机、选中、点击、拖动等多状态动画。
7. 支持为每个软件设置自定义图标；选择 exe 路径后会自动尝试提取软件图标，也可以手动选择图标。
8. 文件夹和网站快捷入口有独立默认图标。
9. 支持在每个软件卡片中单独勾选是否以管理员身份启动。
10. 软件、文件夹和网站快捷入口保存到 Tauri 应用数据目录的 `apps.json`。
11. 宠物位置、自定义宠物形象、分类、快捷搜索、标签显示模式、AI 接口和窗口置顶设置保存到应用数据目录的 `config.json`。
12. 打开快捷入口时，前端只传 `app_id`，Rust 后端读取 `apps.json` 后按类型启动软件、打开文件夹或打开网站。
13. 系统托盘提供显示宠物、隐藏宠物、打开抽屉和退出程序。
14. 抽屉右上角可以直接切换缩略显示和详细显示。
15. 抽屉设置按入口管理、AI 接口、窗口、更新和诊断分类组织。
16. AI 接口设置支持配置 OpenAI 兼容、DeepSeek、Anthropic、Gemini、Ollama 和自定义服务，用于宠物聊天对话。
17. 最近 7 天内打开 2 次及以上的快捷入口会自动加入“常用”，自动加入的快捷入口超过 7 天未打开会自动移出。

## 普通用户下载和使用步骤

普通用户只需要下载已经打包好的安装包或 exe 文件，不需要安装 Node.js、npm、Rust、Cargo 或 Microsoft C++ Build Tools。

### 下载程序

1. 从发布者提供的下载链接中下载最新版 PetDrawer。
2. 优先下载安装包文件，文件名通常类似：

```txt
PetDrawer_0.2.0_x64-setup.exe
```

3. 如果发布者提供的是免安装版本，文件名通常类似：

```txt
pet_drawer.exe
```

4. 不要下载源码压缩包来直接运行。源码包通常包含 `src`、`src-tauri`、`package.json` 等文件，只适合开发者使用。

### 安装版本使用步骤

1. 双击 `PetDrawer_0.2.0_x64-setup.exe`。
2. 如果 Windows 弹出安全提示，确认文件来自可信发布者后，选择“更多信息”，再选择“仍要运行”。
3. 按安装向导完成安装。
4. 安装完成后，从桌面快捷方式、开始菜单或安装目录启动 PetDrawer。
5. 启动后，桌面上会出现一个宠物窗口。
6. 点击宠物可以打开或隐藏快捷入口抽屉。
7. 右键点击宠物可以打开宠物菜单，选择“对话”可以打开宠物聊天窗口。
8. 通过系统托盘图标可以显示宠物、隐藏宠物、打开抽屉或退出程序。

### 免安装版本使用步骤

1. 将 `pet_drawer.exe` 放到一个固定目录，例如：

```txt
D:\Apps\PetDrawer\
```

2. 双击 `pet_drawer.exe` 启动程序。
3. 不建议把 exe 放在临时目录、下载缓存目录或会被自动清理的目录中。
4. 如果需要创建桌面快捷方式，右键 `pet_drawer.exe`，选择“发送到” -> “桌面快捷方式”。

### 添加快捷入口

1. 启动 PetDrawer 后，点击桌面宠物打开快捷入口抽屉。
2. 点击“添加”，在弹窗顶部选择要添加的软件、文件夹或网站类型。
3. 添加软件时选择本机已有的 `.exe` 程序；添加文件夹时选择本机文件夹；添加网站时填写网址。
4. 填写名称、分类和标签；名称为空时会根据路径或网址自动生成。
5. 保存后，快捷入口会显示在抽屉中。
6. 如果软件需要提权，在软件卡片中勾选“管理员启动”；后续启动时 Windows 会弹出 UAC 确认窗口。
7. 后续点击卡片即可快速打开该软件、文件夹或网站。

### 更换宠物形象

1. 打开快捷入口抽屉。
2. 点击左上角“当前宠物形象”区域中的“更换”。
3. 选择已有宠物，或导入自己的宠物图片。
4. 导入宠物时，至少需要选择一个待机动画图片。
5. 支持的图片格式包括 `png`、`jpg`、`jpeg`、`webp`、`gif`、`ico`。
6. 本地导入的宠物可以在“更换宠物形象”窗口右侧点击“删除形象”删除。
7. 内置默认凯蒂不能删除，避免程序没有可用的默认宠物。

### 数据保存位置

PetDrawer 会把快捷入口列表、设置、图标和导入的宠物保存到系统应用数据目录中。卸载或移动程序前，如果需要保留数据，可以先备份 PetDrawer 的应用数据目录。

保存的数据包括：

```txt
apps.json      软件、文件夹和网站快捷入口列表
config.json    窗口位置、当前宠物、分类、快捷搜索等设置
icons/         自定义软件图标
pets/          导入的宠物形象
```

### 常见问题

1. 如果双击后没有反应，先检查程序是否被杀毒软件拦截。
2. 如果系统提示缺少 WebView2 Runtime，请安装 Microsoft Edge WebView2 Runtime 后再启动。Windows 10/11 通常已经自带。
3. 如果 Windows 提示“已保护你的电脑”，确认文件来源可信后，点击“更多信息” -> “仍要运行”。
4. 如果添加的软件无法启动，检查该软件路径是否仍然存在，或重新编辑软件路径。
5. 如果文件夹无法打开，检查该文件夹是否仍然存在，并确认保存的路径指向文件夹而不是文件。
6. 如果宠物或抽屉位置异常，可以退出程序后重新启动。
7. 如果要彻底退出程序，请使用系统托盘菜单中的“退出”，不要只关闭抽屉窗口。

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

项目会在启动前自动检查并创建 `src-tauri/icons/icon.ico`。脚本会优先使用 `../assets/icons/pet-drawer-icon.ico` 作为软件图标；如果该文件不存在，才会生成内置备用图标。如果你看到 `icons/icon.ico not found`，可以手动执行：

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
    DrawerWindow.vue    快捷入口抽屉窗口
    PetMenuWindow.vue   宠物右键菜单窗口
    PetChatWindow.vue   宠物聊天窗口
  components/           抽屉和宠物 UI 组件
  stores/appStore.ts    快捷入口数据状态管理
  types/app.ts          前端类型
  utils/format.ts       文本格式化工具
src-tauri/
  src/app_data.rs       JSON 数据读写
  src/ai_chat.rs        宠物聊天 AI 请求适配
  src/commands.rs       Tauri 命令
  src/launcher.rs       软件、文件夹和网站打开逻辑
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
  preview.svg    凯蒂展示图
  idle.svg       待机：轻微呼吸并眨眼
  hover.svg      选中：挥右手打招呼
  click.svg      点击：开心微笑并冒爱心
  dragging.svg   拖动：身体倾斜，裙摆、尾巴和小脚晃动
```

1. 更换宠物：打开快捷入口抽屉，左上角“当前宠物形象”区域会显示当前宠物，点击“更换”。
2. 搜索已有宠物：更换窗口会先读取应用数据目录下的 `pets/skins/`，并展示每个宠物的预览。
3. 查看宠物信息：点击宠物卡片后，右侧会显示宠物名称、来源和待机/选中/点击/拖动动画配置。
4. 选择宠物：点击预览卡片即可切换，切换结果会写入 `config.json`，重启后仍会保留。
5. 导入宠物：在更换窗口下方“导入宠物”区域填写名称，并选择动画图片。
6. 删除宠物：选中本地导入的宠物后，点击右侧“删除形象”；如果删除的是当前正在使用的宠物，会自动切回内置默认凯蒂。
7. 动画规范：`待机动画` 必填，`选中动画`、`点击动画`、`拖动动画` 可选；未设置的状态会自动使用待机动画。
8. 支持格式：`png`、`jpg`、`jpeg`、`webp`、`gif`、`ico`。
9. 导入后的文件会复制到应用数据目录的 `pets/skins/<skin_id>/`，并生成 `pet.json`。
10. 上传 GitHub 时只需要提交项目内置的 `src/assets/pets/default/` 默认凯蒂；导入的其他宠物属于本机应用数据，不会随仓库上传。

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
2. 抽屉右上角“设置”按钮旁边可以直接切换“缩略/详细”显示方式。
3. 在“入口管理”中新增或删除分类；`全部`、`常用`、`其他` 为核心分类，会自动保留，文件夹和网站作为入口类型不再作为分类显示。
4. 在“入口管理”中新增或删除快捷搜索按钮；这些按钮会显示在搜索框下方。
5. 在“AI 接口”中配置服务商、Base URL、模型、API Key、系统提示词、温度和最大输出 Token；这些配置会保存到本机 `config.json`，用于宠物聊天对话。
6. 在“窗口”中分别控制宠物窗口和抽屉窗口是否始终置顶。
7. 在“更新”中查看当前版本，并检查发布者是否配置了新版下载入口。
8. 在“诊断”中查看当前 exe 路径和本机数据目录。

## 宠物对话

1. 先打开抽屉设置，在“AI 接口”中启用宠物聊天 API。
2. 选择服务商并填写 Base URL、模型和 API Key；Ollama 本地服务通常可以不填 API Key。
3. 右键点击宠物，在弹出的菜单中选择“对话”。
4. 在宠物聊天窗口中输入消息，按 Enter 或点击“发送”即可对话。
5. 如果 AI 接口未启用或配置不完整，聊天窗口会提示先打开抽屉设置补全配置。

## 自动常用规则

1. 快捷入口每次打开成功后，会记录一次打开时间。
2. 最近 7 天内打开 2 次及以上的快捷入口，会自动进入“常用”分类。
3. 自动进入“常用”的快捷入口，如果超过 7 天没有再次打开，会自动从“常用”分类移出。
4. 手动勾选“设为常用”的快捷入口不会被自动移出。
