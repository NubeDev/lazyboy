/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_RPC?: "mock" | "tauri" | "http";
  readonly VITE_CORE_URL?: string;
  readonly VITE_CORE_TOKEN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
