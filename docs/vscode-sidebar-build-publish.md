# VS Code 侧边栏扩展打包与发布

本文只覆盖本仓库里的 VS Code 侧边栏扩展：

- 扩展壳目录：`src/features/sidebar/extension/`
- 侧边栏前端入口源码：`src/entries/sidebar.html`
- 侧边栏前端源码：`src/features/sidebar/`

扩展壳同时承担两件事：

1. 注册 VS Code Activity Bar 里的 Pai 侧边栏 Webview，并把 discovery 信息注入给侧边栏前端连接 `/chat`。
2. 同步 VS Code 当前可见编辑器、选区和可见范围到 PAI 的 `/ide-context`，供侧边栏和桌面端发送前作为 IDE 引用块附加。

## 先说结论

这个扩展不是单独构建的。

`src/entries/sidebar.html` 是 `vite.config.ts` 里的多入口源码之一，所以必须先在仓库根目录执行一次 `pnpm build`，让根 `dist/` 产出侧边栏页面和对应 assets。然后再把根 `dist/` 同步到 `src/features/sidebar/extension/dist/`，最后才能打 `.vsix` 或发布到 Marketplace。

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

> **注意**：Azure DevOps Personal Access Token (PAT) 于 2026 年 12 月 1 日停用，以下流程使用 Microsoft Entra ID 方式发布，无需 PAT。

### 一次性准备

1. **创建 Publisher**

   访问 [https://marketplace.visualstudio.com/manage/publishers/](https://marketplace.visualstudio.com/manage/publishers/)，用微软账号登录，点 **Create publisher**。

   - **ID**：填入 `yokami233618`（必须与 `package.json` 的 `publisher` 字段一致）
   - **Name**：显示名称

2. **确认扩展清单**

   检查 [src/features/sidebar/extension/package.json](/src/features/sidebar/extension/package.json) 的 `publisher` 字段是否与上一步创建的一致。

### 一键发布

```bash
pnpm publish:vscode-sidebar
```

脚本会先打包，再用 `--azure-credential` 模式发布。首次执行会弹出浏览器窗口，用微软账号登录授权。

#### 常用参数

| 参数 | 作用 |
|------|------|
| `-SkipBuild` | 跳过前端构建，用现有 dist/ |
| `-SkipPackage` | 跳过打包，用已有的 `.vsix` 直接发布 |
| `-SkipDuplicate` | 跳过重复版本检查（版本已存在时不报错） |
| `-PreRelease` | 发预发布版 |

### 手动发布（绕过脚本）

如果已有 `.vsix` 文件，也可以直接跑：

```bash
pnpm dlx @vscode/vsce publish --packagePath src/features/sidebar/extension/pai-test.vsix --azure-credential --allow-missing-repository --skip-license
```

### 如果发布脚本还没适配 `--azure-credential`

当前脚本 `scripts/publish-vscode-sidebar.ps1` 还在用 `VSCE_PAT`，在脚本更新前可以先手动发布：

```bash
pnpm package:vscode-sidebar
pnpm dlx @vscode/vsce publish --packagePath src/features/sidebar/extension/pai-test.vsix --azure-credential --allow-missing-repository --skip-license
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
