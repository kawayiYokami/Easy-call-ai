export const CHAT_INPUT_COMPOSITION_CONFIRM_WINDOW_MS = 100;

export interface ChatInputEnterConfirmsCompositionEvent {
  isComposing: boolean;
  keyCode: number;
  key: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}

/**
 * 判断 Enter 是否属于「输入法组合确认回车」。
 *
 * 背景：macOS WebKit（Safari/WKWebView）在输入法确认候选词时事件顺序颠倒
 * （compositionend 先于 keydown），导致 keydown 到达时 isComposing 已为 false；
 * mac 自带中文输入法下 keyCode 甚至不是 229，而是 13。因此除标准两层判断
 * （isComposing / keyCode 229 / 自维护 composing 标志）外，还需要
 * compositionend 后短时间窗口兜底。
 *
 * 关键约束：输入法确认键永远是裸 Enter（无修饰键），因此带修饰键的 Enter
 * （Ctrl/Shift/Alt/Meta）不可能是 IME 确认，直接放行——保证
 * Ctrl+Enter 发送模式等组合快捷键不受时间窗口干扰。
 */
export function chatInputEnterConfirmsComposition(
  event: ChatInputEnterConfirmsCompositionEvent,
  composing: boolean,
  compositionEndedAt: number,
  now: number,
): boolean {
  if (event.isComposing || event.keyCode === 229 || composing) return true;
  if (event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) return false;
  if (event.key !== "Enter" || compositionEndedAt === 0) return false;
  return now - compositionEndedAt < CHAT_INPUT_COMPOSITION_CONFIRM_WINDOW_MS;
}
