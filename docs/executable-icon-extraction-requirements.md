# 抽屉高清软件图标提取功能需求计划

## 背景

当前抽屉添加软件入口时，选择 `.exe` 后会自动调用后端提取关联图标，并保存到本机图标目录。现有实现依赖 `[System.Drawing.Icon]::ExtractAssociatedIcon()`，能满足基础可用性，但图标清晰度、`.lnk` 快捷方式解析和多层回退能力有限。

## 目标

1. 添加或编辑软件入口时，选择 `.exe` 后自动提取更高清的软件图标。
2. 手动“选择图标”入口支持选择图片、`.ico`、`.exe` 和 `.lnk`。
3. 优先从可执行文件或图标文件资源中读取高质量图标，失败时使用 Windows Shell 图标。
4. 自动提取结果只保存到用户本机图标目录，不写入源码、示例文档或仓库。
5. 前端保持现有交互结构，默认主题和 `animal-island` 主题继续正常显示图标预览、按钮、加载态和错误提示。

## 非目标

1. 不上传 exe、快捷方式、图标或文件路径到外部服务。
2. 不把用户选择的软件路径、快捷方式路径或图标文件路径写入仓库文档示例。
3. 不实现跨平台自动提取；非 Windows 仍提示当前仅支持 Windows 自动获取软件图标。
4. 不引入音频指纹、图像识别或远程图标识别服务。

## 实现方案

1. 后端继续复用现有 `import_executable_icon` Tauri 命令，避免新增前端数据结构。
2. Windows 下将图标输出格式从 `.ico` 调整为 `.png`，保存为 `icons/auto_icon_<时间戳>.png`。
3. 用内嵌 PowerShell + C# 调用 Windows API：
   - `PrivateExtractIcons` 优先从 exe 或 ico 资源取 256 图标。
   - `IShellItemImageFactory` 从 Shell 取高质量图标位图。
   - `SHGetFileInfo` + `SHGetImageList(SHIL_JUMBO)` 作为 jumbo 系统图标回退。
4. `.lnk` 快捷方式通过 `WScript.Shell` 解析 `TargetPath` 和 `IconLocation`，支持 `path,index` 图标索引。
5. 前端手动选择图标时，根据扩展名分流：
   - 图片扩展名继续使用 `import_app_icon`。
   - `.exe`、`.lnk`、`.ico` 使用 `import_executable_icon`。

## 隐私要求

1. 图标提取只在本机运行。
2. 软件路径、快捷方式路径和导入图标只进入用户本机应用数据目录中的 `apps.json` 与 `icons/`。
3. 不在源码、需求文档、README 或 Release notes 中记录用户真实路径。
4. 错误提示只描述失败原因，不展示不必要的完整本机路径。

## 主题检查

1. 默认主题：确认软件图标预览、选择按钮和加载态可继承现有 `.app-modal` / `.icon-picker` 样式。
2. `animal-island` 主题：确认软件图标预览、选择按钮和加载态可继承现有主题覆盖样式。
3. 本次不新增窗口、弹窗或独立样式类；如后续新增错误提示样式，需要同步覆盖两个主题。

## 当前实现状态

### 已完成

- [x] 需求计划文档。
- [x] 项目级已完成/未完成功能记录。
- [x] 后端高清图标提取。
- [x] 手动选择图标支持 `.exe` / `.lnk` / `.ico`。
- [x] 默认主题和 `animal-island` 主题继承检查。
- [x] 类型检查或构建验证。

### 待实现

- [ ] 非 Windows 平台自动提取替代方案。
- [ ] 多尺寸图标手动选择。
- [ ] 图标提取失败时展示更细分的用户可操作建议。
