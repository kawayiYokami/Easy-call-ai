import { invoke } from "@tauri-apps/api/core";

export function isTauriRuntimeAvailable(): boolean {
  if (typeof window === "undefined") return false;
  const internals = (window as Window & { __TAURI_INTERNALS__?: { invoke?: unknown } }).__TAURI_INTERNALS__;
  return typeof internals?.invoke === "function";
}

export function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}
