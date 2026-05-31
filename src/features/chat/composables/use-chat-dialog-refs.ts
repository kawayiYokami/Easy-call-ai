import type { Ref } from "vue";

type UseChatDialogRefsOptions = {
  memoryDialog: Ref<HTMLDialogElement | null>;
  promptPreviewDialog: Ref<HTMLDialogElement | null>;
};

export function useChatDialogRefs(options: UseChatDialogRefsOptions) {
  function setMemoryDialogRef(el: Element | null) {
    options.memoryDialog.value = (el as HTMLDialogElement | null) ?? null;
  }

  function setPromptPreviewDialogRef(el: Element | null) {
    options.promptPreviewDialog.value = (el as HTMLDialogElement | null) ?? null;
  }

  return {
    setMemoryDialogRef,
    setPromptPreviewDialogRef,
  };
}
