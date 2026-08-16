# 未发布

## 功能

- 新增 `windows` 窗口管理工具：`list windows` 可列出全部可见窗口（含当前应用自身，含标题/进程/位置/最小化/聚焦状态），`activate window id=xxx` 可将目标窗口还原并切换到前台，便于配合截图定位与操作。Windows、Linux（X11）与 macOS 可用（macOS 需在系统设置中授予辅助功能权限）。
- 桌面脚本的截图命令新增 `elements=true` 参数：配合局部截图（region/focused_window）使用时，会返回截图区域内的可交互控件列表（控件类型、名称与归一化坐标），便于精确点击。区域外的控件会被过滤，只查看画面时无需开启。Windows、Linux（X11，AT-SPI2）与 macOS（需辅助功能权限）可用。

## 重构

## 修复

## 性能

## 依赖
