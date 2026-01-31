/**
 * Client Provider for DeliDev
 *
 * This component provides the client configuration context to the app,
 * handling initialization and mode switching between single-process
 * and remote client modes.
 */

import React, { createContext, useContext, useEffect, useState, useCallback } from "react";
import type { ClientMode } from "./hooks";
import {
  getStoredConfig,
  initializeClient,
  switchToRemoteMode,
  switchToSingleProcessMode,
  testServerConnection,
  getAuthToken,
  setAuthToken as storeAuthToken,
  clearAuthToken,
  isAuthenticated as checkIsAuthenticated,
} from "./client-config";

// ========== Context Types ==========

export interface ClientContextValue {
  /** Current client mode */
  mode: ClientMode;
  /** Whether the client is initialized */
  isInitialized: boolean;
  /** Whether the client is loading */
  isLoading: boolean;
  /** Current server URL (if in remote mode) */
  serverUrl: string | null;
  /** Whether the user is authenticated */
  isAuthenticated: boolean;
  /** Error message if initialization failed */
  error: string | null;
  /** Switch to remote mode */
  connectToServer: (serverUrl: string) => Promise<void>;
  /** Switch to single process mode */
  disconnectFromServer: () => Promise<void>;
  /** Test server connection */
  testConnection: (serverUrl: string) => Promise<boolean>;
  /** Set authentication token */
  setAuthToken: (token: string) => void;
  /** Clear authentication token */
  logout: () => void;
}

// ========== Context ==========

const ClientContext = createContext<ClientContextValue | null>(null);

// ========== Provider Component ==========

export interface ClientProviderProps {
  children: React.ReactNode;
}

export function ClientProvider({ children }: ClientProviderProps) {
  const [mode, setMode] = useState<ClientMode>("single_process");
  const [isInitialized, setIsInitialized] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [serverUrl, setServerUrl] = useState<string | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Initialize client on mount
  useEffect(() => {
    async function init() {
      setIsLoading(true);
      setError(null);

      try {
        await initializeClient();

        const config = getStoredConfig();
        setMode(config.mode);
        setServerUrl(config.serverUrl ?? null);
        setIsAuthenticated(checkIsAuthenticated());
        setIsInitialized(true);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to initialize client");
      } finally {
        setIsLoading(false);
      }
    }

    init();
  }, []);

  // Connect to remote server
  const connectToServer = useCallback(async (url: string) => {
    setIsLoading(true);
    setError(null);

    try {
      // Test connection first
      const isConnected = await testServerConnection(url);
      if (!isConnected) {
        throw new Error("Could not connect to server. Please check the URL and try again.");
      }

      await switchToRemoteMode(url);
      setMode("remote");
      setServerUrl(url);
      setIsAuthenticated(checkIsAuthenticated());
    } catch (e) {
      const message = e instanceof Error ? e.message : "Failed to connect to server";
      setError(message);
      throw e;
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Disconnect from remote server
  const disconnectFromServer = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      await switchToSingleProcessMode();
      setMode("single_process");
      setServerUrl(null);
      setIsAuthenticated(true); // Always authenticated in single process mode
    } catch (e) {
      const message = e instanceof Error ? e.message : "Failed to disconnect";
      setError(message);
      throw e;
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Test server connection
  const testConnection = useCallback(async (url: string): Promise<boolean> => {
    return testServerConnection(url);
  }, []);

  // Set authentication token
  const handleSetAuthToken = useCallback((token: string) => {
    storeAuthToken(token);
    setIsAuthenticated(true);
  }, []);

  // Clear authentication token (logout)
  const logout = useCallback(() => {
    clearAuthToken();
    setIsAuthenticated(mode === "single_process");
  }, [mode]);

  const contextValue: ClientContextValue = {
    mode,
    isInitialized,
    isLoading,
    serverUrl,
    isAuthenticated,
    error,
    connectToServer,
    disconnectFromServer,
    testConnection,
    setAuthToken: handleSetAuthToken,
    logout,
  };

  return (
    <ClientContext.Provider value={contextValue}>
      {children}
    </ClientContext.Provider>
  );
}

// ========== Hook ==========

/**
 * Hook to access the client context
 */
export function useClient(): ClientContextValue {
  const context = useContext(ClientContext);

  if (!context) {
    throw new Error("useClient must be used within a ClientProvider");
  }

  return context;
}

// ========== Utility Components ==========

/**
 * Component that only renders when client is initialized
 */
export function WhenInitialized({ children }: { children: React.ReactNode }) {
  const { isInitialized, isLoading } = useClient();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  if (!isInitialized) {
    return null;
  }

  return <>{children}</>;
}

/**
 * Component that only renders in single process mode
 */
export function WhenSingleProcess({ children }: { children: React.ReactNode }) {
  const { mode } = useClient();

  if (mode !== "single_process") {
    return null;
  }

  return <>{children}</>;
}

/**
 * Component that only renders in remote mode
 */
export function WhenRemote({ children }: { children: React.ReactNode }) {
  const { mode } = useClient();

  if (mode !== "remote") {
    return null;
  }

  return <>{children}</>;
}

/**
 * Component that only renders when authenticated
 */
export function WhenAuthenticated({ children }: { children: React.ReactNode }) {
  const { isAuthenticated } = useClient();

  if (!isAuthenticated) {
    return null;
  }

  return <>{children}</>;
}
