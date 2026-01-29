/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly PUBLIC_APP_VERSION?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
