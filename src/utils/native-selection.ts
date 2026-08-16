const INSTALL_KEY = "__easyCallNativeSelectionGuardInstalled";
const DIALOG_PATCH_KEY = "__easyCallNativeSelectionDialogPatched";
const DIALOG_OBSERVER_KEY = "__easyCallNativeSelectionDialogObserver";

const EXPLICIT_OPEN_TRIGGER_SELECTOR = "[data-clear-selection-before-open]";

const EDITABLE_SELECTOR = [
  "textarea",
  "select",
  "option",
  "input:not([type='button']):not([type='submit']):not([type='reset'])",
  "[contenteditable]:not([contenteditable='false'])",
].join(",");

type GuardedWindow = Window & typeof globalThis & {
  [INSTALL_KEY]?: boolean;
  [DIALOG_PATCH_KEY]?: boolean;
  [DIALOG_OBSERVER_KEY]?: MutationObserver;
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
  if (target.closest(EDITABLE_SELECTOR)) return false;
  return !!target.closest(EXPLICIT_OPEN_TRIGGER_SELECTOR);
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

function isModalElement(node: Element): boolean {
  return node instanceof HTMLDialogElement || node.classList.contains("modal");
}

function isModalVisiblyOpen(element: Element): boolean {
  return (
    (element instanceof HTMLDialogElement && element.open)
    || element.classList.contains("modal-open")
  );
}

function installDialogOpenObserver(win: GuardedWindow) {
  if (win[DIALOG_OBSERVER_KEY]) return;

  const modalOpenStates = new WeakMap<Element, boolean>();
  const rememberModal = (modal: Element) => {
    modalOpenStates.set(modal, isModalVisiblyOpen(modal));
  };
  const handleModalStateChange = (modal: Element) => {
    const wasOpen = modalOpenStates.get(modal) ?? false;
    const isOpen = isModalVisiblyOpen(modal);
    modalOpenStates.set(modal, isOpen);
    if (!wasOpen && isOpen) {
      clearNativeTextSelection();
    }
  };
  const handleAddedModal = (modal: Element) => {
    const isOpen = isModalVisiblyOpen(modal);
    modalOpenStates.set(modal, isOpen);
    if (isOpen) {
      clearNativeTextSelection();
    }
  };
  const handleAddedNode = (node: Node) => {
    if (!(node instanceof Element)) return;
    if (isModalElement(node)) {
      handleAddedModal(node);
    }
    node.querySelectorAll("dialog, .modal").forEach((modal) => {
      if (isModalElement(modal)) {
        handleAddedModal(modal);
      }
    });
  };

  document.querySelectorAll("dialog, .modal").forEach((modal) => {
    if (isModalElement(modal)) {
      rememberModal(modal);
    }
  });

  const observer = new MutationObserver((records) => {
    for (const record of records) {
      if (record.type === "attributes" && record.target instanceof Element && isModalElement(record.target)) {
        handleModalStateChange(record.target);
        continue;
      }
      record.addedNodes.forEach(handleAddedNode);
    }
  });

  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["class", "open"],
    childList: true,
    subtree: true,
  });
  win[DIALOG_OBSERVER_KEY] = observer;
}

export function installNativeSelectionGuard() {
  if (typeof window === "undefined" || typeof document === "undefined") return;

  const win = window as GuardedWindow;
  patchDialogOpenMethods(win);
  installDialogOpenObserver(win);

  if (win[INSTALL_KEY]) return;
  document.addEventListener("pointerdown", clearBeforeInteractiveAction, true);
  document.addEventListener("click", clearBeforeInteractiveAction, true);
  document.addEventListener("keydown", clearBeforeKeyboardAction, true);
  win[INSTALL_KEY] = true;
}
