import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import { Loader2, Server, Monitor, ArrowRight, CheckCircle2 } from "lucide-react";
import {
  saveConfig,
  testServerConnection,
  switchToSingleProcessMode,
  switchToRemoteMode,
} from "../api/client-config";

export enum ConnectionMode {
  Local = "local",
  Server = "server",
}

const MODE_SELECTION_STORAGE_KEY = "delidev_mode_selected";

/**
 * Checks if mode has been selected before
 */
export function hasModeBeenSelected(): boolean {
  return localStorage.getItem(MODE_SELECTION_STORAGE_KEY) === "true";
}

/**
 * Marks mode as selected
 */
export function markModeAsSelected(): void {
  localStorage.setItem(MODE_SELECTION_STORAGE_KEY, "true");
}

/**
 * Clears mode selection (for dev mode to re-show selection)
 */
export function clearModeSelection(): void {
  localStorage.removeItem(MODE_SELECTION_STORAGE_KEY);
}

export function ModeSelection() {
  const navigate = useNavigate();

  const [selectedMode, setSelectedMode] = useState<ConnectionMode | null>(null);
  const [serverUrl, setServerUrl] = useState("");
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectionError, setConnectionError] = useState<string | null>(null);
  const [connectionSuccess, setConnectionSuccess] = useState(false);

  const handleModeSelect = (mode: ConnectionMode) => {
    setSelectedMode(mode);
    setConnectionError(null);
    setConnectionSuccess(false);
  };

  const handleTestConnection = async () => {
    if (!serverUrl) return;

    setIsConnecting(true);
    setConnectionError(null);
    setConnectionSuccess(false);

    try {
      const isConnected = await testServerConnection(serverUrl);
      if (isConnected) {
        setConnectionSuccess(true);
      } else {
        setConnectionError("Could not connect to server. Please check the URL and try again.");
      }
    } catch (error) {
      setConnectionError(
        error instanceof Error ? error.message : "Connection test failed"
      );
    } finally {
      setIsConnecting(false);
    }
  };

  const handleContinue = async () => {
    if (!selectedMode) return;

    setIsConnecting(true);
    setConnectionError(null);

    try {
      if (selectedMode === ConnectionMode.Local) {
        await switchToSingleProcessMode();
      } else {
        if (!serverUrl) {
          setConnectionError("Please enter a server URL");
          setIsConnecting(false);
          return;
        }
        await switchToRemoteMode(serverUrl);
      }

      markModeAsSelected();
      navigate("/onboarding");
    } catch (error) {
      setConnectionError(
        error instanceof Error ? error.message : "Failed to set mode"
      );
    } finally {
      setIsConnecting(false);
    }
  };

  const canContinue =
    selectedMode === ConnectionMode.Local ||
    (selectedMode === ConnectionMode.Server && serverUrl && connectionSuccess);

  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <Card className="w-full max-w-lg">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">Welcome to DeliDev</CardTitle>
          <CardDescription>
            Choose how you want to run DeliDev
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-6">
            <div className="space-y-4">
              {/* Local Mode Option */}
              <div
                className={`border-2 rounded-lg p-4 cursor-pointer transition-all ${
                  selectedMode === ConnectionMode.Local
                    ? "border-primary bg-primary/5"
                    : "border-border hover:border-primary/50"
                }`}
                onClick={() => handleModeSelect(ConnectionMode.Local)}
              >
                <div className="flex items-start gap-4">
                  <div
                    className={`p-2 rounded-lg ${
                      selectedMode === ConnectionMode.Local
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted"
                    }`}
                  >
                    <Monitor className="h-6 w-6" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium">Local Mode</h3>
                    <p className="text-sm text-muted-foreground mt-1">
                      Run everything locally on your machine. All processing happens on your computer with no external server required.
                    </p>
                    <ul className="text-xs text-muted-foreground mt-2 space-y-1">
                      <li>• Full privacy - your code never leaves your machine</li>
                      <li>• No network latency</li>
                      <li>• Works offline (requires local AI setup)</li>
                    </ul>
                  </div>
                </div>
              </div>

              {/* Server Mode Option */}
              <div
                className={`border-2 rounded-lg p-4 cursor-pointer transition-all ${
                  selectedMode === ConnectionMode.Server
                    ? "border-primary bg-primary/5"
                    : "border-border hover:border-primary/50"
                }`}
                onClick={() => handleModeSelect(ConnectionMode.Server)}
              >
                <div className="flex items-start gap-4">
                  <div
                    className={`p-2 rounded-lg ${
                      selectedMode === ConnectionMode.Server
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted"
                    }`}
                  >
                    <Server className="h-6 w-6" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-medium">Server Mode</h3>
                    <p className="text-sm text-muted-foreground mt-1">
                      Connect to a remote DeliDev server for task execution and coordination.
                    </p>
                    <ul className="text-xs text-muted-foreground mt-2 space-y-1">
                      <li>• Centralized task management</li>
                      <li>• Team collaboration support</li>
                      <li>• Offload computation to server</li>
                    </ul>
                  </div>
                </div>
              </div>
            </div>

            {/* Server URL Input (only shown when Server mode is selected) */}
            {selectedMode === ConnectionMode.Server && (
              <div className="space-y-4 pt-2 border-t">
                <div className="space-y-2">
                  <Label htmlFor="server-url">Server URL</Label>
                  <Input
                    id="server-url"
                    type="url"
                    value={serverUrl}
                    onChange={(e) => {
                      setServerUrl(e.target.value);
                      setConnectionSuccess(false);
                      setConnectionError(null);
                    }}
                    placeholder="https://delidev.example.com"
                  />
                  <p className="text-xs text-muted-foreground">
                    Enter the URL of your DeliDev server
                  </p>
                </div>

                <Button
                  variant="outline"
                  className="w-full"
                  onClick={handleTestConnection}
                  disabled={!serverUrl || isConnecting}
                >
                  {isConnecting ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : connectionSuccess ? (
                    <>
                      <CheckCircle2 className="h-4 w-4 text-green-500" />
                      Connected
                    </>
                  ) : (
                    "Test Connection"
                  )}
                </Button>

                {connectionError && (
                  <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-3">
                    <p className="text-sm text-destructive">{connectionError}</p>
                  </div>
                )}

                {connectionSuccess && (
                  <div className="rounded-lg border border-green-500/50 bg-green-50 p-3">
                    <p className="text-sm text-green-800">
                      Successfully connected to server
                    </p>
                  </div>
                )}
              </div>
            )}

            <div className="pt-4 border-t">
              <Button
                className="w-full"
                onClick={handleContinue}
                disabled={!canContinue || isConnecting}
              >
                {isConnecting ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <>
                    Continue
                    <ArrowRight className="h-4 w-4" />
                  </>
                )}
              </Button>
              <p className="text-xs text-muted-foreground text-center mt-3">
                You can change this setting later in Settings
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
