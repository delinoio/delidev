import { defineConfig } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";

export default defineConfig({
  plugins: [pluginReact()],
  source: {
    entry: {
      index: "./src/main.tsx",
    },
    define: {
      // Expose environment variables to the client
      // These can be set via environment or command line
      "import.meta.env.PUBLIC_DEFAULT_MODE": JSON.stringify(
        process.env.PUBLIC_DEFAULT_MODE || "local"
      ),
      "import.meta.env.PUBLIC_REMOTE_SERVER_URL": JSON.stringify(
        process.env.PUBLIC_REMOTE_SERVER_URL || ""
      ),
      "import.meta.env.PUBLIC_SKIP_MODE_SELECTION": JSON.stringify(
        process.env.PUBLIC_SKIP_MODE_SELECTION || ""
      ),
    },
  },
  html: {
    template: "./index.html",
  },
  output: {
    distPath: {
      root: "dist",
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
