# 20260324 read_file 工具与 litchi 实验接入计划

## 1. 背景

当前项目内置工具已覆盖网页抓取、截图、命令、终端执行、补丁编辑、任务与委托，但还没有一个统一的“本地文件读取”能力。

本次目标不是一次性把所有文件格式都做到稳定完备，而是先建立一个稳定的 `read_file` 工具协议和 Rust 侧抽象接口，让后续不同格式的解析实现都能挂到同一入口下。

结合当前仓库现状与实验结果：

- PDF 已有现成稳定实现，可直接复用。
- 文本文件与图片读取可以直接由 Rust 处理。
- Office 格式中，`litchi` 在当前样本上已验证：
  - `.doc` 可读取
  - `.xlsx` 可读取
  - `.ppt` 会 panic，当前不能视为稳定实现

因此本次更适合采用“抽象先行、provider 化接入、风险隔离”的策略。

## 2. 目标

### 2.1 功能目标

1. 新增内置工具 `read_file`。
2. 工具参数遵循约定：
   - `absolute_path`
   - `offset`
   - `limit`
3. 首版返回统一结构，支持分页、截断提示与后续继续读取。
4. Rust 侧建立统一文件读取抽象接口，不把格式细节散落在工具层。
5. 文本、图片、PDF 先接稳定实现。
6. Office 解析通过独立 provider 接入，首版允许 `litchi` 作为实验实现参与。
7. `litchi` 相关调用必须做 panic 隔离，不能让 `.ppt` 之类的异常直接炸掉聊天运行时。

### 2.2 非目标

1. 本次不承诺所有 Office 格式都达到生产级稳定。
2. 本次不实现复杂的附件索引、缓存数据库或全文索引。
3. 本次不新增 Java sidecar、Node sidecar 或外部 Office 运行时。
4. 本次不做目录浏览器或文件选择 UI，只做已有工具体系内的文件读取能力。

## 3. 工具协议

### 3.1 输入

- `absolute_path: string`，必填，必须是绝对路径
- `offset?: number`，可选，文本类按 0-based 行号分页
- `limit?: number`，可选，限制本次返回的最大行数

### 3.2 输出

建议返回统一 JSON 结构，至少包含：

- `ok`
- `absolutePath`
- `detectedType`
- `readerKind`
- `truncated`
- `nextOffset`
- `content`
- `metadata`

其中：

- `detectedType` 表示格式判断结果，如 `text` / `image` / `pdf` / `doc` / `docx` / `xls` / `xlsx` / `ppt` / `pptx`
- `readerKind` 表示最终命中的读取器，如 `text` / `pdf_builtin` / `litchi`
- `content` 按不同类型承载文本或图片数据
- `metadata` 用于补充页数、字符数、工作表数量、是否实验实现等

### 3.3 文本上限

- 文本总内容上限 30,000 字符
- 超出后返回 `truncated=true`
- 若是按行分页命中的文本文件，优先保留 `nextOffset`

## 4. Rust 侧抽象设计

### 4.1 核心结构

新增一组统一类型，例如：

- `ReadFileRequest`
- `ReadFileResult`
- `ReadFileContent`
- `ReadFileMetadata`
- `ReadFileReader` trait

建议 trait 形态：

1. `supports(path: &Path, detected: &DetectedFileType) -> bool`
2. `read(request: &ReadFileRequest) -> Result<ReadFileResult, String>`

### 4.2 Reader 分层

建议拆为：

- `TextFileReader`
- `ImageFileReader`
- `PdfFileReader`
- `OfficeLitchiReader`

后续如果要引入：

- `OfficeNodeReader`
- `OfficeJavaPoiReader`
- `DocxNativeReader`

都可以继续挂到同一层。

## 5. 各格式首版策略

### 5.1 文本文件

- 直接按 UTF-8 文本读取
- 支持 `offset + limit`
- 行级分页优先
- 非 UTF-8 文本先返回清晰错误，不做编码猜测扩展

### 5.2 图片

- 支持：`png/jpg/jpeg/gif/webp/svg/bmp`
- 返回统一图片结果结构
- 小图可直接返回 base64 与 mime
- 不做 OCR，本次只是读取图片本体

### 5.3 PDF

- 直接复用现有 `pdf_text_service`
- 走文本抽取模式
- 按页文本拼接后再套统一截断策略

### 5.4 Office

首版统一由 `OfficeLitchiReader` 承担实验接入：

- `doc`
- `docx`
- `xls`
- `xlsx`
- `ppt`
- `pptx`

但运行时策略要分级：

- `doc/xls/xlsx`：按实验能力开放
- `docx/pptx`：若本地验证通过则开放
- `ppt`：即使接入，也必须加 panic 保护与降级错误

## 6. litchi 接入策略

### 6.1 依赖策略

- 使用固定 `git rev`
- 不直接跟踪 `main`
- 标记为实验性实现

### 6.2 风险控制

由于当前本地已复现 `.ppt` panic：

- 所有 `litchi` 调用必须包在 `std::panic::catch_unwind` 中
- 若发生 panic，统一转换为可读错误
- 错误信息中要提示“实验性 Office reader 失败”

### 6.3 当前样本验证结论

基于 `E:\github\easy_call_ai\data`：

- `.doc`：成功
- `.xlsx`：成功
- `.ppt`：检测成功，但解析 panic

因此首版实现不能把 `litchi` 视为完全稳定依赖。

## 7. 工具接入点

本次实现至少需要同步这几处：

- 运行时工具装配：
  - `src-tauri/src/features/chat/model_runtime/provider_and_stream/tool_assembly.rs`
- 工具参数与封装：
  - `src-tauri/src/features/chat/model_runtime/tools_and_builtin/tool_arg_types.rs`
  - `src-tauri/src/features/chat/model_runtime/tools_and_builtin/tool_impls.rs`
- 前端工具目录：
  - `src-tauri/src/features/system/commands/chat_and_runtime/tool_catalog.rs`
- 默认工具配置：
  - `src-tauri/src/features/core/domain.rs`
  - `src/features/config/utils/builtin-tools.ts`
- 状态检查：
  - `src-tauri/src/features/system/commands/chat_and_runtime/tools_and_cache.rs`

实际读取实现建议新增独立模块，避免把文件读取逻辑塞进现有超大文件。

## 8. 实现顺序

### 第一阶段：抽象骨架

1. 定义请求/结果模型
2. 定义 reader trait
3. 建立读取调度入口
4. 接入内置工具定义，但先只连稳定 reader

### 第二阶段：稳定 reader

1. 文本 reader
2. 图片 reader
3. PDF reader

### 第三阶段：Office 实验 reader

1. 接入 `litchi`
2. 补充格式分发
3. 为 `ppt` 增加 panic 隔离
4. 用样本验证 `doc/xlsx/ppt`

### 第四阶段：前端目录与状态同步

1. 工具列表显示 `read_file`
2. 默认配置中加入 `read_file`
3. 状态检查中标注实验/可用状态

## 9. 验收点

1. `read_file` 能出现在工具目录与运行时工具清单中。
2. 文本文件支持整文件读取与分页读取。
3. 图片文件能返回 mime 与内容。
4. PDF 能复用现有文本提取结果。
5. `doc/xlsx` 在当前样本上能通过 `read_file` 返回文本。
6. `ppt` 即使失败，也只返回结构化错误，不会 panic 传播到主流程。
7. 工具返回内容超过 30,000 字符时会明确截断并提示继续读取。

## 10. 风险与注意事项

1. `litchi` 官方明确仍处于 active development，不建议按生产级稳定依赖对待。
2. `ppt` 已有本地 panic 样本，必须隔离。
3. Office 文本抽取结果的格式化质量可能不统一，首版优先保证“能读出文本”，不追求完美排版。
4. 绝对路径输入需要严守校验，避免把相对路径、空路径或非法路径放进运行时。
5. 文本/二进制/Office 的返回结构必须统一，否则后续工具提示词会变复杂。
