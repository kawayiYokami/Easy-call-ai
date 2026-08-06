# 未发布

## 重构

## 修复

- 远程访问密码认证：被 iframe 嵌入（远程前端模式）时优先向父窗口请求已保存密码，避免 Android WebView 拦截跨域 iframe 的 window.prompt 导致认证卡住（桌面独立窗口行为不变）
- 远程前端模式窗口栏叠加：被 iframe 嵌入且非 VSCode 宿主时隐藏页面自带窗口栏，避免与手机壳层 header 上下叠加（桌面独立窗口与 VSCode 侧边栏行为不变）
- 远程通知标题语言：广播 assistantDelta 附带的会话标题改为跟随用户 ui_language 配置（不再写死 zh-CN），与本地 live update 通知标题保持一致
- 远程桥接安全加固：与 iframe 父窗口（手机壳层）的 postMessage 双向通信统一校验约定 origin，转发 targetOrigin 不再使用通配 `*`，防恶意页面伪造密码/会话命令或窃听通知事件
- 远程桥接安全加固：密码消息接收侧额外校验 event.source 必须等于 window.parent，拒绝同 origin 下其他窗口伪造的密码注入
- 远程通知标题测试：会话标题广播测试覆盖全部非本地会话过滤分支（delegate / remote-IM / system-notification），并验证标题跟随用户 ui_language 配置（en-US 兜底文案）

## 依赖

## 功能
