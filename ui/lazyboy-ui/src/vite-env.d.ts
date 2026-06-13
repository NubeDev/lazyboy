/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_USE_MOCK?: "1";
  readonly VITE_CORE_URL?: string;
  readonly VITE_CORE_TOKEN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
