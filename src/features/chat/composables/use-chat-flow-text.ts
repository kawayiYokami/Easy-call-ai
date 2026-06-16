export function mergeAssistantText(currentText: string, finalText: string): string {
  const current = String(currentText || "");
  const finalValue = String(finalText || "");
  if (!current) return finalValue;
  if (!finalValue) return current;
  if (finalValue.startsWith(current)) return finalValue;
  return finalValue;
}

export function hasAssistantVisibleOutput(result: {
  assistantText: string;
}): boolean {
  return !!result.assistantText.trim();
}

export function consumeClosedMarkdownBlocks(input: string): { chunks: string[]; tail: string } {
  // 乐观渲染策略：直接返回所有内容作为 chunks，tail 为空
  // 这样所有 markdown 元素（标题、粗体、引用等）都能立即渲染
  if (!input) return { chunks: [], tail: "" };

  return { chunks: [input], tail: "" };
}
