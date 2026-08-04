# 未发布

## 重构

## 修复

- 远程访问密码认证：被 iframe 嵌入（远程前端模式）时优先向父窗口请求已保存密码，避免 Android WebView 拦截跨域 iframe 的 window.prompt 导致认证卡住（桌面独立窗口行为不变）
- 远程前端模式窗口栏叠加：被 iframe 嵌入且非 VSCode 宿主时隐藏页面自带窗口栏，避免与手机壳层 header 上下叠加（桌面独立窗口与 VSCode 侧边栏行为不变）

## 依赖

## 功能
