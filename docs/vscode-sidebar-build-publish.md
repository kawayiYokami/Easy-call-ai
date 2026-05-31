# VS Code 侧边栏扩展打包与发布

本文只覆盖本仓库里的 VS Code 侧边栏扩展：

- 扩展壳目录：`src/features/sidebar/extension/`
- 侧边栏前端入口：`sidebar.html`
- 侧边栏前端源码：`src/features/sidebar/`

扩展壳同时承担两件事：

1. 注册 VS Code Activity Bar 里的 Pai 侧边栏 Webview，并把 discovery 信息注入给侧边栏前端连接 `/chat`。
2. 同步 VS Code 当前可见编辑器、选区和可见范围到 PAI 的 `/ide-context`，供侧边栏和桌面端发送前作为 IDE 引用块附加。

## 先说结论

这个扩展不是单独构建的。

`sidebar.html` 是根 `vite.config.ts` 里的多入口之一，所以必须先在仓库根目录执行一次 `pnpm build`，让根 `dist/` 产出侧边栏页面和对应 assets。然后再把根 `dist/` 同步到 `src/features/sidebar/extension/dist/`，最后才能打 `.vsix` 或发布到 Marketplace。

现在仓库已经提供了一键命令：

```bash
pnpm package:vscode-sidebar
pnpm publish:vscode-sidebar
```

## 一键打包

直接在仓库根目录执行：

```bash
pnpm package:vscode-sidebar
```

这个命令会自动做三件事：

1. 执行根目录 `pnpm build`
2. 把根 `dist/` 同步到 `src/features/sidebar/extension/dist/`
3. 在扩展目录生成 `pai-test.vsix`

默认产物位置：

```text
src/features/sidebar/extension/pai-test.vsix
```

如果要自定义输出文件名，可以这样传参：

```bash
pnpm package:vscode-sidebar -- -OutputPath pai-0.9.93.vsix
```

如果你已经手动跑过 `pnpm build`，想跳过再次构建：

```bash
pnpm package:vscode-sidebar -- -SkipBuild
```

## 发布到 VS Code 商店

### 一次性准备

1. 在 Visual Studio Marketplace 创建 Publisher

   官方页面：
   `https://marketplace.visualstudio.com/manage/publishers/`

2. 确认扩展清单里的 `publisher` 字段和你创建的 Publisher ID 完全一致

   当前文件：
   `src/features/sidebar/extension/package.json`

3. 用同一个 Microsoft 账号去 Azure DevOps 创建 PAT

   官方文档要求使用 Marketplace 的管理权限。当前 `vsce publish --help` 也明确说明可以通过 `--pat` 或 `VSCE_PAT` 环境变量提供 token。

4. 给 PAT 打开 `Marketplace > Manage` 权限

5. 在当前终端设置环境变量

PowerShell 当前会话：

```powershell
$env:VSCE_PAT = "your-token"
```

如果想写到用户环境变量：

```powershell
setx VSCE_PAT "your-token"
```

注意：`setx` 之后要开一个新的终端窗口才会生效。

### 一键发布

在仓库根目录执行：

```bash
pnpm publish:vscode-sidebar
```

这个命令会先自动打包，再执行 Marketplace 发布。

如果你已经有现成的 `.vsix`，只想发布不重新打包：

```bash
pnpm publish:vscode-sidebar -- -SkipPackage
```

如果你要发预发布版本：

```bash
pnpm publish:vscode-sidebar -- -PreRelease
```

如果担心重复版本导致脚本失败：

```bash
pnpm publish:vscode-sidebar -- -SkipDuplicate
```

## 这套脚本背后的实际命令

打包脚本内部等价于：

```bash
pnpm build
pnpm dlx @vscode/vsce package -o pai-test.vsix --allow-missing-repository --skip-license
```

发布脚本内部等价于：

```bash
pnpm dlx @vscode/vsce publish --packagePath <vsix-path> --pat $VSCE_PAT --allow-missing-repository --skip-license
```

## 当前仓库的注意事项

- 现在扩展目录没有单独的 `repository` 元数据和 `LICENSE` 文件，所以脚本里临时带了 `--allow-missing-repository` 和 `--skip-license`
- 这对内部测试和先发版本够用，但如果要长期公开维护，最好后续把扩展自己的 README、CHANGELOG、LICENSE、repository 信息补齐
- 扩展设置页只保留两个用户意图开关：`paiSidebar.autoSendIdeContext` 控制是否自动同步，`paiSidebar.includeVisibleRange` 控制无选区时是否同步可见代码
- IDE 上下文会在编辑器变化时自动同步，并用低频 heartbeat 续租静止窗口，避免 PAI 侧 TTL 清掉仍在线的 VS Code 引用
- 官方文档还要求：
  - `package.json` 里的扩展图标不能用 SVG
  - `README.md` / `CHANGELOG.md` 里的图片链接应该是 `https`
  - 用户提供的 SVG 图片不能直接用于发布包

## 官方参考

- Publishing Extensions
  - https://code.visualstudio.com/api/working-with-extensions/publishing-extension
- Extension Manifest
  - https://code.visualstudio.com/api/references/extension-manifest
- Bundling Extensions
  - https://code.visualstudio.com/api/working-with-extensions/bundling-extension
