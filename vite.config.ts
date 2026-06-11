import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "node:path";
import { readdir, rename, rm } from "node:fs/promises";

const entryDir = resolve(__dirname, "src/entries");
const htmlEntryAliases = new Set([
  "/",
  "/index.html",
  "/chat.html",
  "/archives.html",
  "/quick-setup.html",
  "/file-reader.html",
  "/runtime-logs.html",
  "/sidebar.html",
  "/settings.html",
]);

export default defineConfig({
  plugins: [
    vue({
      template: {
        compilerOptions: {
          isCustomElement: (tag) => tag.startsWith("calendar-"),
        },
      },
    }),
    {
      name: "pai-html-entry-rewrite",
      configureServer(server) {
        server.middlewares.use((req, _res, next) => {
          const requestUrl = req.url ?? "/";
          const [pathname, search = ""] = requestUrl.split("?", 2);
          if (!htmlEntryAliases.has(pathname)) {
            next();
            return;
          }

          const normalizedPath = pathname === "/" ? "/index.html" : pathname;
          req.url = `/src/entries${normalizedPath}${search ? `?${search}` : ""}`;
          next();
        });
      },
      async closeBundle() {
        const builtEntryDir = resolve(__dirname, "dist/src/entries");
        let builtEntries: string[] = [];
        try {
          builtEntries = await readdir(builtEntryDir);
        } catch {
          return;
        }

        await Promise.all(
          builtEntries
            .filter((fileName) => fileName.endsWith(".html"))
            .map((fileName) =>
              rename(
                resolve(builtEntryDir, fileName),
                resolve(__dirname, "dist", fileName),
              ),
            ),
        );

        await rm(resolve(__dirname, "dist/src"), { recursive: true, force: true });
      },
    },
  ],
  clearScreen: false,
  build: {
    rollupOptions: {
      input: {
        config: resolve(entryDir, "index.html"),
        chat: resolve(entryDir, "chat.html"),
        archives: resolve(entryDir, "archives.html"),
        quickSetup: resolve(entryDir, "quick-setup.html"),
        fileReader: resolve(entryDir, "file-reader.html"),
        runtimeLogs: resolve(entryDir, "runtime-logs.html"),
        sidebar: resolve(entryDir, "sidebar.html"),
        settings: resolve(entryDir, "settings.html"),
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: [
        "**/.git/**",
        "**/node_modules/**",
        "**/dist/**",
        "**/src-tauri/target/**",
        "**/src-tauri/memory/**",
        "**/src-tauri/gen/**",
        "**/src-tauri/icons/**",
      ],
    },
  },
});
