const INSTALL_KEY = "__easyCallNativeSelectionGuardInstalled";
const DIALOG_PATCH_KEY = "__easyCallNativeSelectionDialogPatched";

const INTERACTIVE_SELECTOR = [
  "button",
  "summary",
  "label",
  "a[href]",
  "input[type='button']",
  "input[type='submit']",
  "input[type='reset']",
  "[role='button']",
  "[data-clear-selection-before-open]",
].join(",");

type GuardedWindow = Window & typeof globalThis & {
  [INSTALL_KEY]?: boolean;
  [DIALOG_PATCH_KEY]?: boolean;
};

export function clearNativeTextSelection() {
  try {
    const selection = window.getSelection?.();
    if (selection && selection.rangeCount > 0) {
      selection.removeAllRanges();
    }
  } catch {
    // WebView2 can crash when a modal opens while native text is selected.
  }
}

function shouldClearSelectionForTarget(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target.closest("[data-preserve-native-selection]")) return false;
  return !!target.closest(INTERACTIVE_SELECTOR);
}

function clearBeforeInteractiveAction(event: Event) {
  if (event instanceof PointerEvent && event.button !== 0) return;
  if (!shouldClearSelectionForTarget(event.target)) return;
  clearNativeTextSelection();
}

function clearBeforeKeyboardAction(event: KeyboardEvent) {
  if (event.key !== "Enter" && event.key !== " ") return;
  clearBeforeInteractiveAction(event);
}

function patchDialogOpenMethods(win: GuardedWindow) {
  if (win[DIALOG_PATCH_KEY]) return;
  const dialogPrototype = win.HTMLDialogElement?.prototype;
  if (!dialogPrototype) return;

  const rawShow = dialogPrototype.show;
  const rawShowModal = dialogPrototype.showModal;

  if (typeof rawShow === "function") {
    dialogPrototype.show = function show(this: HTMLDialogElement) {
      clearNativeTextSelection();
      return rawShow.call(this);
    };
  }

  if (typeof rawShowModal === "function") {
    dialogPrototype.showModal = function showModal(this: HTMLDialogElement) {
      clearNativeTextSelection();
      return rawShowModal.call(this);
    };
  }

  win[DIALOG_PATCH_KEY] = true;
}

export function installNativeSelectionGuard() {
  if (typeof window === "undefined" || typeof document === "undefined") return;

  const win = window as GuardedWindow;
  patchDialogOpenMethods(win);

  if (win[INSTALL_KEY]) return;
  document.addEventListener("pointerdown", clearBeforeInteractiveAction, true);
  document.addEventListener("click", clearBeforeInteractiveAction, true);
  document.addEventListener("keydown", clearBeforeKeyboardAction, true);
  win[INSTALL_KEY] = true;
}
