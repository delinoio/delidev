/// <reference types="@rsbuild/core/types" />

interface ImportMetaEnv {
  readonly PUBLIC_APP_VERSION?: string;
  readonly DEV: boolean;
  readonly PROD: boolean;
  readonly MODE: string;
  /**
   * Default connection mode for the app.
   * "local" - Use single-process mode (default)
   * "remote" - Connect to a remote server
   */
  readonly PUBLIC_DEFAULT_MODE?: "local" | "remote";
  /**
   * Remote server URL (required when PUBLIC_DEFAULT_MODE is "remote")
   */
  readonly PUBLIC_REMOTE_SERVER_URL?: string;
  /**
   * Skip mode selection screen and use the configured mode directly
   */
  readonly PUBLIC_SKIP_MODE_SELECTION?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
