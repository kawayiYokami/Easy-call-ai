/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>;
  export default component;
}

declare module "markdown-it" {
  interface MarkdownIt {
    render(src: string, env?: unknown): string;
  }
  interface MarkdownItConstructor {
    new (options?: Record<string, unknown>): MarkdownIt;
    (options?: Record<string, unknown>): MarkdownIt;
  }
  const MarkdownIt: MarkdownItConstructor;
  export default MarkdownIt;
}
