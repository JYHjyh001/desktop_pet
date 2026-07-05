# 划选翻译 PDF 剪贴板格式增强需求计划

状态：第二阶段已实现，增强福昕阅读器等 PDF 阅读器兼容性。

本文档规划划选翻译在 PDF 阅读器、浏览器 PDF 预览、Office 类阅读窗口中的兼容性增强。该增强仍属于独立翻译功能，不保存选中文本、译文、来源窗口、PDF 路径或剪贴板历史。

## 背景

当前划选翻译通过全局快捷键触发后模拟 `Ctrl+C`，再读取 Windows 剪贴板中的纯文本格式 `CF_UNICODETEXT`。部分 PDF 阅读器复制选区时不会写入纯文本，而是只写入 HTML、RTF 或应用自定义格式，导致桌宠无法读取到可翻译文本。

## 目标

1. 在保留现有纯文本读取路径的基础上，增加 HTML Clipboard Format 读取和纯文本提取。
2. 增加 Rich Text Format 读取和基础纯文本提取。
3. 对 PDF 阅读器复制出的段落、换行、列表文本做轻量归一化，减少多余空白。
4. 失败提示继续说明目标应用可能没有提供可复制文本。
5. 不记录、不持久化、不上传除用户本次翻译请求以外的剪贴板内容。

## 第二阶段目标：福昕阅读器兼容

1. 增加 `CF_TEXT` 和 `CF_OEMTEXT` 标准纯文本格式读取，用于兼容只写入 ANSI/OEM 文本的 PDF 阅读器。
2. 增加 `Rich Text Format Without Objects` 等 RTF 变体读取，用于兼容部分富文本复制实现。
3. HTML/RTF 原始字节增加 UTF-8、UTF-16LE 和 UTF-16BE 解码兜底，避免因编码差异导致正文提取失败。
4. 复制后轮询时间放宽，并对剪贴板短暂占用做重试，避免福昕阅读器复制较慢或短暂锁住剪贴板时误判失败。
5. 失败提示可返回当前剪贴板格式摘要，帮助判断福昕是否只提供了不可解析的私有格式；摘要不包含 PDF 路径、窗口标题或正文。

## 非目标

1. 第一阶段不做 OCR、截图翻译或图片型 PDF 识别。
2. 第一阶段不接入 Windows UI Automation、IAccessible 或 PDF 私有 API。
3. 不绕过 PDF 复制限制、权限限制或 DRM。
4. 不保存 PDF 文件路径、窗口标题、应用名称或复制失败诊断日志。
5. 不完整保留非文本剪贴板格式；当前仍只恢复文本剪贴板内容。
6. 第二阶段仍不绕过福昕或 PDF 文档自身的禁止复制、权限限制和 DRM；扫描件或图片型 PDF 仍需要后续 OCR 功能。

## 技术方案

1. Windows 剪贴板读取顺序：
   - `CF_UNICODETEXT`
   - 注册格式 `HTML Format`
   - 注册格式 `Rich Text Format`
   - 第二阶段补充：`CF_TEXT`、`CF_OEMTEXT`、`Rich Text Format Without Objects`
2. HTML Clipboard Format：
   - 优先读取 `StartFragment` / `EndFragment` 指定片段。
   - 移除标签并解码常见 HTML entity。
   - 将 `<br>`、段落、列表项等块级标签转换为换行。
3. RTF：
   - 支持基础控制词解析。
   - 支持 `\uN` Unicode 转义、`\par`、`\line`、`\tab`。
   - 跳过 `fonttbl`、`colortbl`、`stylesheet`、`pict` 等非正文目的地。
4. 选中文本校验继续只拦截明显 UUID、长哈希、URL 或路径，不拦截普通英文句子。
5. 失败诊断只展示格式名和通用操作建议，不展示剪贴板正文、PDF 文件路径或来源窗口标题。

## 验收标准

1. 普通文本编辑器选中英文仍可翻译。
2. 写入剪贴板的 HTML Format 能被提取为纯文本。
3. 写入剪贴板的 RTF 能被提取为纯文本。
4. PDF 阅读器若 `Ctrl+C` 写入 HTML 或 RTF，划选翻译可以读取正文。
5. 福昕阅读器若写入 `CF_TEXT`、`CF_OEMTEXT` 或 RTF 变体，划选翻译可以读取正文。
6. 扫描件 PDF 或禁止复制的 PDF 仍给出可理解失败提示，并可提示检测到的剪贴板格式摘要。
7. `cargo test translation`、`cargo check`、`npm run build` 通过。

## 后续阶段

1. Windows UI Automation/IAccessible 读取当前选区文本。
2. 截图 OCR 兜底，用于扫描件 PDF 和图片内容。
3. 更完整的剪贴板格式恢复，减少对用户原剪贴板非文本内容的影响。
