# Windows exe 图标提取通用方法

本文档整理一套可迁移到其他桌面项目的 Windows 软件图标提取方案。适用于 Tauri、Electron、Rust、Node.js 或其他能调用本机 PowerShell 的桌面应用。

## 适用场景

1. 用户选择 `.exe` 后，应用自动提取软件图标。
2. 用户选择 `.lnk` 快捷方式后，应用提取快捷方式指向的软件图标。
3. 用户选择 `.ico` 后，应用转换或提取为前端更容易预览的 PNG。
4. 前端需要展示软件图标预览，后端需要把图标保存到本机应用数据目录。

## 总体流程

1. 前端打开文件选择器，让用户选择图标来源文件。
2. 前端只做轻量扩展名校验，允许：
   - `exe`
   - `lnk`
   - `ico`
   - 常规图片：`png`、`jpg`、`jpeg`、`webp`、`gif`
3. 如果是常规图片，直接复制到应用数据目录。
4. 如果是 `exe`、`lnk` 或 `ico`，调用后端图标提取命令。
5. 后端启动 `powershell.exe`，执行内嵌 PowerShell + C# 代码。
6. C# 通过 Windows API 提取图标，并保存为 PNG。
7. 后端返回应用数据目录中的相对路径，或返回 data URL 给前端预览。

## 推荐输出格式

统一输出 PNG，而不是 ICO。

原因：

1. PNG 更适合 WebView、HTML、Canvas 和普通 `<img>` 预览。
2. ICO 内部可能包含多尺寸图标，前端直接预览时兼容性和清晰度不稳定。
3. 应用可以统一保存为：

```text
icons/auto_icon_<timestamp>.png
```

## 前端文件选择建议

不要强依赖文件对话框的扩展名过滤器来显示 `.exe`。某些 Windows/Tauri 对话框组合过滤器会导致进入文件夹后看不到或无法选择 exe。

推荐做法：

1. 文件对话框不强制过滤，或只提供非常宽松的过滤。
2. 用户选中后，再用代码校验扩展名。
3. 不支持的文件显示明确错误提示。

示例：

```ts
const imageFileExtensions = ['png', 'jpg', 'jpeg', 'webp', 'gif', 'ico']
const executableIconSourceExtensions = ['exe', 'lnk', 'ico']
const iconSourceFileExtensions = Array.from(
  new Set([...imageFileExtensions, ...executableIconSourceExtensions]),
)

function fileExtension(path: string) {
  const fileName = path.replace(/\\/g, '/').split('/').pop() ?? ''
  const index = fileName.lastIndexOf('.')
  return index >= 0 ? fileName.slice(index + 1).toLowerCase() : ''
}
```

选择后分流：

```ts
const extension = fileExtension(selected)
if (!iconSourceFileExtensions.includes(extension)) {
  throw new Error('请选择 png、jpg、jpeg、webp、gif、ico、exe 或 lnk 文件作为图标来源')
}

const command = executableIconSourceExtensions.includes(extension)
  ? 'import_executable_icon'
  : 'import_app_icon'
```

## 后端校验

后端必须再次校验文件存在和扩展名，不要只信任前端。

建议规则：

1. `source_path` 必须指向本机文件。
2. 自动提取只允许：
   - `exe`
   - `lnk`
   - `ico`
3. 输出文件必须写入应用数据目录下的图标目录。
4. 返回给前端时优先返回相对路径，不返回不必要的本机绝对路径。

## Windows API 回退顺序

推荐使用以下顺序：

### 1. 解析 lnk 快捷方式

如果输入是 `.lnk`，先用 `WScript.Shell` 解析：

1. `TargetPath`
2. `IconLocation`
3. `WorkingDirectory`

`IconLocation` 可能是：

```text
<icon-file-path>,<icon-index>
```

需要拆成图标文件路径和图标索引。

### 2. 从资源中提取图标

优先调用：

```text
PrivateExtractIcons
```

请求尺寸建议：

```text
256 x 256
```

适合从 exe 或 ico 的资源表中提取原始高清图标。

### 3. 从 Shell 获取图标位图

资源提取失败后调用：

```text
SHCreateItemFromParsingName
IShellItemImageFactory.GetImage
```

这个方式接近 Windows 资源管理器显示图标的逻辑，兼容性好。

### 4. 从系统 Jumbo 图标列表回退

Shell 位图失败后调用：

```text
SHGetFileInfo
SHGetImageList(SHIL_JUMBO)
IImageList.GetIcon
```

这是最后一层本地图标回退。

## PowerShell 调用方式

建议显式调用 Windows PowerShell：

```text
powershell.exe
```

不要默认依赖 `pwsh`。`pwsh` 与 Windows PowerShell 在 `System.Drawing` 相关程序集加载上行为不同，容易出现兼容问题。

推荐调用参数：

```text
-NoProfile -ExecutionPolicy Bypass -Command <script-block> <source> <target>
```

脚本块建议使用 `param` 接收参数：

```powershell
& {
  param([string]$source, [string]$target)
  # icon extraction logic
}
```

不要直接依赖 `$args`，复杂脚本通过 `-Command` 传参时容易出现参数错位或解析异常。

## 资源释放要求

提取图标时会涉及 Windows 句柄，必须释放：

1. `HICON` 使用后调用 `DestroyIcon`。
2. `HBITMAP` 使用后调用 `DeleteObject`。
3. `Icon`、`Bitmap`、`Graphics` 等 .NET 对象使用后调用 `Dispose()` 或放进 `using`。

否则长期导入图标可能造成句柄泄漏。

## Tauri 接入方式

Tauri / Rust 中可以用：

```rust
std::process::Command::new("powershell.exe")
```

Windows 下建议加：

```rust
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;
command.creation_flags(CREATE_NO_WINDOW);
```

这样执行 PowerShell 时不会弹出黑色窗口。

## Electron 接入方式

Electron 主进程中可以用：

```js
spawn("powershell.exe", [
  "-NoProfile",
  "-ExecutionPolicy",
  "Bypass",
  "-Command",
  command,
  sourcePath,
  outputPath
], { windowsHide: true })
```

如果 PowerShell 脚本打包在 `app.asar` 中，不能直接当文件执行。推荐：

1. 启动时从 asar 读取脚本文本。
2. 写入系统临时目录。
3. 再用 `powershell.exe -File <temp-script>` 执行。

如果脚本直接内嵌在主进程字符串中，则不需要额外解压。

## 隐私和数据边界

1. 图标提取只在用户本机运行。
2. 不上传 exe、lnk、ico、图片或文件路径。
3. 用户选择的软件路径属于本机私有数据，只能保存到应用运行时数据中。
4. 提取出的图标也属于用户本机数据，只保存到应用数据目录。
5. 文档、README、示例和测试不要写入用户真实路径。
6. 错误提示应避免展示不必要的完整本机路径。

## 测试清单

至少测试以下输入：

1. 系统自带 exe，例如 `<WindowsDir>/System32/notepad.exe`。
2. 普通第三方软件 exe。
3. 指向 exe 的 `.lnk` 快捷方式。
4. 带 `IconLocation` 的 `.lnk` 快捷方式。
5. `.ico` 文件。
6. 不支持的文件类型。
7. 图标提取失败时的错误提示。

验收标准：

1. 能生成 PNG 文件。
2. PNG 宽高大于 0，推荐能拿到 256 x 256。
3. 前端 `<img>` 可正常预览。
4. 输出文件位于应用数据目录。
5. 不会产生黑色终端窗口。
6. 不会把本机路径写入源码、文档或仓库。

## 常见问题

### 文件选择器看不到 exe

优先移除文件对话框过滤器，改为选择后校验扩展名。

### PowerShell 里 System.Drawing 报错

确认调用的是 `powershell.exe`，不是 `pwsh`。

### -Command 传参错位

使用：

```powershell
& {
  param([string]$source, [string]$target)
}
```

不要把复杂脚本直接拼成依赖 `$args` 的命令。

### ICO 预览不清晰

后端统一转 PNG，再交给前端预览。

## PetDrawer 当前参考实现

当前项目中的可参考位置：

1. 后端提取入口：`src-tauri/src/app_data.rs` 中的 `import_executable_icon`
2. Windows 提取实现：`src-tauri/src/app_data.rs` 中的 `extract_associated_icon`
3. 前端选择图标入口：`src/windows/DrawerWindow.vue` 中的 `pickAppIcon`

迁移到其他项目时，优先复制思路和调用链，不要复制任何用户本机路径或运行时数据。
