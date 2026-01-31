/**
 * Client Configuration for DeliDev
 *
 * This module handles the configuration for switching between single-process
 * mode (local desktop app) and client mode (connecting to remote server).
 */

import { invoke } from "@tauri-apps/api/core";
import { setClientMode, type ClientMode } from "./hooks";
import { initRpcClient, resetRpcClient } from "./rpc";

// ========== Configuration Types ==========

export interface ServerConfig {
  /** Server mode: single_process or remote */
  mode: ClientMode;
  /** Remote server URL (required when mode is "remote") */
  serverUrl?: string;
}

export interface AppConfig {
  server: ServerConfig;
}

// ========== Storage Keys ==========

const STORAGE_KEY = "delidev_client_config";

// ========== Configuration Management ==========

/**
 * Gets the stored client configuration
 */
export function getStoredConfig(): ServerConfig {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored) as ServerConfig;
    }
  } catch {
    // Ignore parse errors
  }

  // Default to single process mode
  return { mode: "single_process" };
}

/**
 * Saves the client configuration
 */
export function saveConfig(config: ServerConfig): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
}

/**
 * Initializes the client based on stored configuration
 */
export async function initializeClient(): Promise<void> {
  const config = getStoredConfig();

  if (config.mode === "remote" && config.serverUrl) {
    // Initialize RPC client for remote mode
    initRpcClient({
      serverUrl: config.serverUrl,
    });

    // Set the client mode for hooks
    setClientMode({
      mode: "remote",
      serverUrl: config.serverUrl,
    });
  } else {
    // Reset to single process mode
    resetRpcClient();
    setClientMode({ mode: "single_process" });
  }
}

/**
 * Switches to remote server mode
 */
export async function switchToRemoteMode(serverUrl: string): Promise<void> {
  // Validate server URL
  if (!serverUrl.startsWith("http://") && !serverUrl.startsWith("https://")) {
    throw new Error("Server URL must start with http:// or https://");
  }

  // Save configuration
  saveConfig({ mode: "remote", serverUrl });

  // Initialize RPC client
  initRpcClient({ serverUrl });

  // Update client mode
  setClientMode({
    mode: "remote",
    serverUrl,
  });
}

/**
 * Switches to single process mode
 */
export async function switchToSingleProcessMode(): Promise<void> {
  // Save configuration
  saveConfig({ mode: "single_process" });

  // Reset RPC client
  resetRpcClient();

  // Update client mode
  setClientMode({ mode: "single_process" });
}

/**
 * Tests connection to a remote server
 */
export async function testServerConnection(serverUrl: string): Promise<boolean> {
  try {
    const response = await fetch(`${serverUrl}/health`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
      },
    });

    return response.ok;
  } catch {
    return false;
  }
}

// ========== Tauri Integration ==========

/**
 * Gets the server mode from Tauri backend
 * This is used to check if the app was started with a specific mode
 */
export async function getTauriServerMode(): Promise<ServerConfig | null> {
  try {
    // Check if we have a Tauri command for getting server mode
    const mode = await invoke<{ mode: ClientMode; serverUrl?: string }>("get_server_mode");
    return mode;
  } catch {
    // If the command doesn't exist or fails, return null
    return null;
  }
}

/**
 * Initializes the client from Tauri configuration if available
 */
export async function initializeFromTauri(): Promise<boolean> {
  const tauriConfig = await getTauriServerMode();

  if (tauriConfig) {
    if (tauriConfig.mode === "remote" && tauriConfig.serverUrl) {
      await switchToRemoteMode(tauriConfig.serverUrl);
    } else {
      await switchToSingleProcessMode();
    }
    return true;
  }

  return false;
}

// ========== Authentication ==========

const AUTH_TOKEN_KEY = "delidev_auth_token";

/**
 * Gets the stored authentication token
 */
export function getAuthToken(): string | null {
  return localStorage.getItem(AUTH_TOKEN_KEY);
}

/**
 * Stores the authentication token
 */
export function setAuthToken(token: string): void {
  localStorage.setItem(AUTH_TOKEN_KEY, token);

  // Update RPC client if in remote mode
  const config = getStoredConfig();
  if (config.mode === "remote" && config.serverUrl) {
    const { getRpcClient } = require("./rpc");
    try {
      const client = getRpcClient();
      client.setAuthToken(token);
    } catch {
      // Client not initialized yet
    }
  }
}

/**
 * Clears the authentication token
 */
export function clearAuthToken(): void {
  localStorage.removeItem(AUTH_TOKEN_KEY);

  // Update RPC client if in remote mode
  const config = getStoredConfig();
  if (config.mode === "remote" && config.serverUrl) {
    const { getRpcClient } = require("./rpc");
    try {
      const client = getRpcClient();
      client.setAuthToken(null);
    } catch {
      // Client not initialized yet
    }
  }
}

/**
 * Checks if the user is authenticated
 */
export function isAuthenticated(): boolean {
  const config = getStoredConfig();

  // In single process mode, no authentication is required
  if (config.mode === "single_process") {
    return true;
  }

  // In remote mode, check for auth token
  return !!getAuthToken();
}
