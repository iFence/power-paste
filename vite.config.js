import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const devHost = process.env.POWER_PASTE_DEV_HOST || "127.0.0.1";
const devPort = Number(process.env.POWER_PASTE_DEV_PORT || "5173");

export default defineConfig({
  plugins: [vue()],
  server: {
    host: devHost,
    port: devPort,
    strictPort: true,
  },
});
