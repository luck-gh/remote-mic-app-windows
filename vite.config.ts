import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { execFileSync } from "node:child_process";
import path from "node:path";
import type { Plugin } from "vite";

function frontendBuildReceipt(): Plugin {
  let root = "";
  let fingerprint = "";
  return {
    name: "frontend-build-receipt",
    apply: "build",
    configResolved(config) {
      root = config.root;
      if (path.resolve(root, config.build.outDir) !== path.join(root, "dist")) {
        throw new Error("frontend_directory_must_be_current_dist");
      }
      fingerprint = JSON.parse(execFileSync(process.execPath, [path.join(root, "scripts/verify-frontend-embedded.mjs"), "fingerprint"], { encoding: "utf8" }));
    },
    writeBundle(_options, bundle) {
      const chunks = Object.values(bundle).filter(output => output.type === "chunk").map(chunk => ({
        fileName: chunk.fileName,
        references: [...chunk.imports, ...chunk.dynamicImports, ...chunk.referencedFiles],
      }));
      execFileSync(process.execPath, [path.join(root, "scripts/verify-frontend-embedded.mjs"), "write-receipt", fingerprint], { input: JSON.stringify(chunks), stdio: ["pipe", "pipe", "inherit"] });
    },
  };
}

export default defineConfig({
  plugins: [vue(), frontendBuildReceipt()],
  clearScreen: false,
  server: {
    // 绑定 IPv4 回环：默认只监听 ::1 时，WebView2 走 127.0.0.1 连不上 dev server，
    // 页面会一直停在 document_load（dev-only 配置，不影响生产构建）。
    host: "127.0.0.1",
    port: 2430,
    strictPort: true,
    watch: {
      ignored: ["**/target/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
});
