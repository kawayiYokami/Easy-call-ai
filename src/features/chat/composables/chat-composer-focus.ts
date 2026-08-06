/**
 * 最后活跃的会话输入框（主会话 / 侧边追问）共享状态。
 *
 * 主会话与追问各有独立的 ChatComposerPanel 实例，全局快捷键与粘贴通道
 * 需要知道「用户最后一次使用的是哪个输入框」，避免把图片/文本路由错会话。
 * 两个输入框在 focus/blur 时通过 registerChatComposerFocus 写入/清除本状态。
 */
import { ref } from "vue";

export type ChatComposerScope = "main" | "side";

const activeComposerScope = ref<ChatComposerScope | null>(null);

export function registerChatComposerFocus(scope: ChatComposerScope): void {
  activeComposerScope.value = scope;
}

export function clearChatComposerFocus(scope: ChatComposerScope): void {
  if (activeComposerScope.value === scope) {
    activeComposerScope.value = null;
  }
}

export function getActiveChatComposerScope(): ChatComposerScope | null {
  return activeComposerScope.value;
}
