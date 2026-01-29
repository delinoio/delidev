import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTabParams } from "../hooks";
import { useConfigStore } from "../stores/config";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { Select } from "../components/ui/select";
import { SimpleCheckbox } from "../components/ui/checkbox";
import { PremiumBadge } from "../components/ui/premium-badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import { Loader2, Save, Check, X, Key, RefreshCw, Trash2 } from "lucide-react";
import { AIAgentType, ContainerRuntime, EditorType, LicenseStatus, type GlobalConfig, type LicenseInfo } from "../types";
import { cn } from "../lib/utils";
import * as api from "../api";

const agentTypeOptions = [
  { value: AIAgentType.ClaudeCode, label: "Claude Code" },
  { value: AIAgentType.OpenCode, label: "OpenCode" },
];

const modelOptions = [
  { value: "claude-sonnet-4-20250514", label: "Claude Sonnet 4" },
  { value: "claude-opus-4-20250514", label: "Claude Opus 4" },
  { value: "gpt-4o", label: "GPT-4o" },
];

const editorTypeOptions = [
  { value: EditorType.Vscode, label: "Visual Studio Code" },
  { value: EditorType.Cursor, label: "Cursor" },
  { value: EditorType.Windsurf, label: "Windsurf" },
  { value: EditorType.VscodeInsiders, label: "VSCode Insiders" },
  { value: EditorType.Vscodium, label: "VSCodium" },
];

const containerRuntimeOptions = [
  { value: ContainerRuntime.Docker, label: "Docker" },
  { value: ContainerRuntime.Podman, label: "Podman" },
];

// Helper function to safely log errors without exposing sensitive data
const logError = (message: string, error: unknown): void => {
  const errorMessage = error instanceof Error ? error.message : String(error);
  console.error(message, errorMessage);
};

// Valid settings tabs
enum SettingsTab {
  Global = "global",
  Notifications = "notifications",
  Credentials = "credentials",
  License = "license",
}

const validTabs = Object.values(SettingsTab);

export function Settings() {
  const { tab } = useTabParams<{ tab: string }>();
  const navigate = useNavigate();

  const {
    globalConfig,
    credentialsStatus,
    fetchGlobalConfig,
    fetchCredentialsStatus,
    updateGlobalConfig,
    setGithubToken,
    setGitlabToken,
    setBitbucketCredentials,
    isLoading,
    error,
  } = useConfigStore();

  // Validate tab param and default to "global" if invalid
  const activeTab = validTabs.includes(tab as SettingsTab) ? (tab as SettingsTab) : SettingsTab.Global;

  // Navigate to valid tab if current tab is invalid
  useEffect(() => {
    if (tab && !validTabs.includes(tab as SettingsTab)) {
      navigate(`/settings/${SettingsTab.Global}`, { replace: true });
    }
  }, [tab, navigate]);

  const setActiveTab = (newTab: SettingsTab) => {
    navigate(`/settings/${newTab}`);
  };
  const [localConfig, setLocalConfig] = useState<GlobalConfig | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);

  // Credentials form state
  const [githubToken, setGithubTokenState] = useState("");
  const [gitlabToken, setGitlabTokenState] = useState("");
  const [bitbucketUsername, setBitbucketUsername] = useState("");
  const [bitbucketPassword, setBitbucketPassword] = useState("");
  const [credentialSaving, setCredentialSaving] = useState<string | null>(null);

  // License state
  const [licenseInfo, setLicenseInfo] = useState<LicenseInfo | null>(null);
  const [licenseKey, setLicenseKey] = useState("");
  const [licenseLoading, setLicenseLoading] = useState(false);
  const [licenseError, setLicenseError] = useState<string | null>(null);

  useEffect(() => {
    fetchGlobalConfig();
    fetchCredentialsStatus();
    fetchLicenseInfo();
  }, [fetchGlobalConfig, fetchCredentialsStatus]);

  const fetchLicenseInfo = async () => {
    try {
      const info = await api.getLicenseInfo();
      setLicenseInfo(info);
    } catch (err) {
      logError("Failed to fetch license info:", err);
    }
  };

  useEffect(() => {
    if (globalConfig) {
      setLocalConfig(globalConfig);
    }
  }, [globalConfig]);

  const handleSave = async () => {
    if (!localConfig) return;
    setIsSaving(true);
    try {
      await updateGlobalConfig(localConfig);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 2000);
    } catch (error) {
      logError("Failed to save config:", error);
    } finally {
      setIsSaving(false);
    }
  };

  const handleGithubSave = async () => {
    if (!githubToken) return;
    setCredentialSaving("github");
    try {
      await setGithubToken(githubToken);
      setGithubTokenState("");
    } catch (error) {
      logError("Failed to save GitHub token:", error);
    } finally {
      setCredentialSaving(null);
    }
  };

  const handleGitlabSave = async () => {
    if (!gitlabToken) return;
    setCredentialSaving("gitlab");
    try {
      await setGitlabToken(gitlabToken);
      setGitlabTokenState("");
    } catch (error) {
      logError("Failed to save GitLab token:", error);
    } finally {
      setCredentialSaving(null);
    }
  };

  const handleBitbucketSave = async () => {
    if (!bitbucketUsername || !bitbucketPassword) return;
    setCredentialSaving("bitbucket");
    try {
      await setBitbucketCredentials(bitbucketUsername, bitbucketPassword);
      setBitbucketUsername("");
      setBitbucketPassword("");
    } catch (error) {
      logError("Failed to save Bitbucket credentials:", error);
    } finally {
      setCredentialSaving(null);
    }
  };

  const handleActivateLicense = async () => {
    if (!licenseKey) return;
    setLicenseLoading(true);
    setLicenseError(null);
    try {
      // First try to set license key using validation (for licenses without activation limits)
      // If the license requires activation, this will fail and we fall back to activate
      const info = await api.setLicenseKey(licenseKey);
      setLicenseInfo(info);
      setLicenseKey("");
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      // If validation says activation is required, try activation instead
      if (errorMessage.includes("requires activation")) {
        try {
          const info = await api.activateLicense(licenseKey);
          setLicenseInfo(info);
          setLicenseKey("");
          return;
        } catch (activateErr) {
          const activateErrorMessage = activateErr instanceof Error ? activateErr.message : String(activateErr);
          setLicenseError(activateErrorMessage);
          logError("Failed to activate license:", activateErr);
        }
      } else {
        setLicenseError(errorMessage);
        logError("Failed to set license key:", err);
      }
    } finally {
      setLicenseLoading(false);
    }
  };

  const handleValidateLicense = async () => {
    setLicenseLoading(true);
    setLicenseError(null);
    try {
      const info = await api.validateLicense();
      setLicenseInfo(info);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setLicenseError(errorMessage);
      logError("Failed to validate license:", err);
    } finally {
      setLicenseLoading(false);
    }
  };

  const handleRemoveLicense = async () => {
    setLicenseLoading(true);
    setLicenseError(null);
    try {
      await api.removeLicense();
      setLicenseInfo({ status: LicenseStatus.NotConfigured });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setLicenseError(errorMessage);
      logError("Failed to remove license:", err);
    } finally {
      setLicenseLoading(false);
    }
  };

  const getLicenseStatusColor = (status: LicenseStatus) => {
    switch (status) {
      case LicenseStatus.Active:
        return "text-green-500";
      case LicenseStatus.Expired:
      case LicenseStatus.Invalid:
      case LicenseStatus.Revoked:
        return "text-destructive";
      case LicenseStatus.Pending:
        return "text-yellow-500";
      default:
        return "text-muted-foreground";
    }
  };

  const getLicenseStatusLabel = (status: LicenseStatus) => {
    switch (status) {
      case LicenseStatus.Active:
        return "Active";
      case LicenseStatus.Expired:
        return "Expired";
      case LicenseStatus.Invalid:
        return "Invalid";
      case LicenseStatus.Revoked:
        return "Revoked";
      case LicenseStatus.Pending:
        return "Pending";
      case LicenseStatus.NotConfigured:
        return "Not Configured";
      default:
        return "Unknown";
    }
  };

  const updateConfig = (path: string[], value: unknown) => {
    if (!localConfig) return;

    const newConfig = JSON.parse(JSON.stringify(localConfig));
    let current: Record<string, unknown> = newConfig;
    for (let i = 0; i < path.length - 1; i++) {
      current = current[path[i]] as Record<string, unknown>;
    }
    current[path[path.length - 1]] = value;
    setLocalConfig(newConfig);
  };

  if (isLoading && !globalConfig) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Settings</h1>
        <p className="text-muted-foreground">
          Configure your DeliDev preferences.
        </p>
      </div>

      {error && (
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {/* Tab Navigation */}
      <div className="flex gap-2 border-b">
        <button
          className={cn(
            "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
            activeTab === SettingsTab.Global
              ? "border-primary text-primary"
              : "border-transparent text-muted-foreground hover:text-foreground"
          )}
          onClick={() => setActiveTab(SettingsTab.Global)}
        >
          Global
        </button>
        <button
          className={cn(
            "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
            activeTab === SettingsTab.Notifications
              ? "border-primary text-primary"
              : "border-transparent text-muted-foreground hover:text-foreground"
          )}
          onClick={() => setActiveTab(SettingsTab.Notifications)}
        >
          Notifications
        </button>
        <button
          className={cn(
            "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
            activeTab === SettingsTab.Credentials
              ? "border-primary text-primary"
              : "border-transparent text-muted-foreground hover:text-foreground"
          )}
          onClick={() => setActiveTab(SettingsTab.Credentials)}
        >
          VCS Credentials
        </button>
        <button
          className={cn(
            "px-4 py-2 text-sm font-medium border-b-2 -mb-px transition-colors",
            activeTab === SettingsTab.License
              ? "border-primary text-primary"
              : "border-transparent text-muted-foreground hover:text-foreground"
          )}
          onClick={() => setActiveTab(SettingsTab.License)}
        >
          License
        </button>
      </div>

      {activeTab === SettingsTab.Global && localConfig && (
        <div className="space-y-6">
          {/* Learning Settings */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                Learning
                <PremiumBadge />
              </CardTitle>
              <CardDescription>
                Configure how DeliDev learns from code reviews.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-3">
                <SimpleCheckbox
                  id="autoLearn"
                  checked={localConfig.learning.auto_learn_from_reviews}
                  onCheckedChange={(checked) =>
                    updateConfig(["learning", "auto_learn_from_reviews"], checked)
                  }
                  disabled={licenseInfo?.status !== LicenseStatus.Active}
                />
                <div>
                  <Label htmlFor="autoLearn">Auto-learn from reviews</Label>
                  <p className="text-xs text-muted-foreground">
                    Automatically extract learning points from VCS provider reviews.
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Hotkey Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Hotkey</CardTitle>
              <CardDescription>
                Configure global keyboard shortcuts.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <Label htmlFor="openChat">Open Chat</Label>
                <Input
                  id="openChat"
                  value={localConfig.hotkey.open_chat}
                  onChange={(e) =>
                    updateConfig(["hotkey", "open_chat"], e.target.value)
                  }
                  placeholder="Option+Z"
                />
                <p className="text-xs text-muted-foreground">
                  Global hotkey to open chat window.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Editor Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Editor</CardTitle>
              <CardDescription>
                Configure your preferred code editor for viewing diffs.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <Label htmlFor="editorType">External Editor</Label>
                <Select
                  id="editorType"
                  options={editorTypeOptions}
                  value={localConfig.editor.editor_type}
                  onChange={(e) =>
                    updateConfig(["editor", "editor_type"], e.target.value)
                  }
                />
                <p className="text-xs text-muted-foreground">
                  The editor to use when opening files to view diffs. Make sure the editor is installed and available in your PATH.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Container Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Container</CardTitle>
              <CardDescription>
                Configure container runtime for agent execution.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center gap-3">
                <SimpleCheckbox
                  id="useContainer"
                  checked={localConfig.container.use_container}
                  onCheckedChange={(checked) =>
                    updateConfig(["container", "use_container"], checked)
                  }
                />
                <div>
                  <Label htmlFor="useContainer">Use container for agent execution</Label>
                  <p className="text-xs text-muted-foreground">
                    When enabled, AI agents run in isolated containers. When disabled, agents run directly on the host.
                  </p>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="containerRuntime">Container Runtime</Label>
                <Select
                  id="containerRuntime"
                  options={containerRuntimeOptions}
                  value={localConfig.container.runtime}
                  onChange={(e) =>
                    updateConfig(["container", "runtime"], e.target.value)
                  }
                  disabled={!localConfig.container.use_container}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="socketPath">Socket Path (optional)</Label>
                <Input
                  id="socketPath"
                  value={localConfig.container.socket_path ?? ""}
                  onChange={(e) =>
                    updateConfig(["container", "socket_path"], e.target.value || undefined)
                  }
                  placeholder="Leave empty for default"
                  disabled={!localConfig.container.use_container}
                />
                <p className="text-xs text-muted-foreground">
                  Custom socket path for the container runtime. Leave empty to use the default path.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Agent Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Agent - Planning</CardTitle>
              <CardDescription>
                Settings for CompositeTask planning.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label>Agent Type</Label>
                  <Select
                    options={agentTypeOptions}
                    value={localConfig.agent.planning.type}
                    onChange={(e) =>
                      updateConfig(
                        ["agent", "planning", "type"],
                        e.target.value
                      )
                    }
                  />
                </div>
                <div className="space-y-2">
                  <Label>Model</Label>
                  <Select
                    options={modelOptions}
                    value={localConfig.agent.planning.model}
                    onChange={(e) =>
                      updateConfig(
                        ["agent", "planning", "model"],
                        e.target.value
                      )
                    }
                  />
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Agent - Execution</CardTitle>
              <CardDescription>
                Settings for UnitTask execution and auto-fix.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label>Agent Type</Label>
                  <Select
                    options={agentTypeOptions}
                    value={localConfig.agent.execution.type}
                    onChange={(e) =>
                      updateConfig(
                        ["agent", "execution", "type"],
                        e.target.value
                      )
                    }
                  />
                </div>
                <div className="space-y-2">
                  <Label>Model</Label>
                  <Select
                    options={modelOptions}
                    value={localConfig.agent.execution.model}
                    onChange={(e) =>
                      updateConfig(
                        ["agent", "execution", "model"],
                        e.target.value
                      )
                    }
                  />
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Agent - Chat</CardTitle>
              <CardDescription>
                Settings for chat interface interactions.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label>Agent Type</Label>
                  <Select
                    options={agentTypeOptions}
                    value={localConfig.agent.chat.type}
                    onChange={(e) =>
                      updateConfig(["agent", "chat", "type"], e.target.value)
                    }
                  />
                </div>
                <div className="space-y-2">
                  <Label>Model</Label>
                  <Select
                    options={modelOptions}
                    value={localConfig.agent.chat.model}
                    onChange={(e) =>
                      updateConfig(["agent", "chat", "model"], e.target.value)
                    }
                  />
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Concurrency Settings */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                Concurrency
                <PremiumBadge />
              </CardTitle>
              <CardDescription>
                Limit the maximum number of concurrent agent sessions.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="maxConcurrentSessions">Max Concurrent Sessions</Label>
                <Input
                  id="maxConcurrentSessions"
                  type="number"
                  min="1"
                  placeholder="Unlimited"
                  value={localConfig.concurrency?.max_concurrent_sessions ?? ""}
                  onChange={(e) => {
                    const value = e.target.value;
                    if (value === "") {
                      // Empty value means unlimited
                      updateConfig(
                        ["concurrency", "max_concurrent_sessions"],
                        undefined
                      );
                      return;
                    }
                    const parsed = parseInt(value, 10);
                    // Only update if the value is a valid positive integer
                    if (!isNaN(parsed) && parsed > 0) {
                      updateConfig(
                        ["concurrency", "max_concurrent_sessions"],
                        parsed
                      );
                    }
                    // Invalid values (NaN, 0, negative) are ignored - the input keeps the previous valid value
                  }}
                  disabled={licenseInfo?.status !== LicenseStatus.Active}
                />
                <p className="text-xs text-muted-foreground">
                  Maximum number of agent sessions that can run simultaneously. Leave empty for unlimited.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Save Button */}
          <div className="flex justify-end">
            <Button onClick={handleSave} disabled={isSaving}>
              {isSaving ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : saveSuccess ? (
                <Check className="h-4 w-4" />
              ) : (
                <Save className="h-4 w-4" />
              )}
              {saveSuccess ? "Saved" : "Save Changes"}
            </Button>
          </div>
        </div>
      )}

      {activeTab === SettingsTab.Notifications && localConfig?.notification && (
        <div className="space-y-6">
          {/* Notification Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Desktop Notifications</CardTitle>
              <CardDescription>
                Configure when to receive desktop notifications from DeliDev.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* Master toggle */}
              <div className="flex items-center gap-3 pb-4 border-b">
                <SimpleCheckbox
                  id="notificationsEnabled"
                  checked={localConfig.notification.enabled}
                  onCheckedChange={(checked) =>
                    updateConfig(["notification", "enabled"], checked)
                  }
                />
                <div>
                  <Label htmlFor="notificationsEnabled" className="text-base">
                    Enable desktop notifications
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Receive notifications when AI agents require your attention.
                  </p>
                </div>
              </div>

              {/* Individual notification types */}
              <div className="space-y-3 pl-8">
                <div className="flex items-center gap-3">
                  <SimpleCheckbox
                    id="approvalRequest"
                    checked={localConfig.notification.approval_request}
                    disabled={!localConfig.notification.enabled}
                    onCheckedChange={(checked) =>
                      updateConfig(["notification", "approval_request"], checked)
                    }
                  />
                  <div>
                    <Label
                      htmlFor="approvalRequest"
                      className={cn(
                        !localConfig.notification.enabled &&
                          "text-muted-foreground"
                      )}
                    >
                      Approval requests
                    </Label>
                    <p className="text-xs text-muted-foreground">
                      Notify when AI agent requests approval for a task or plan.
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-3">
                  <SimpleCheckbox
                    id="userQuestion"
                    checked={localConfig.notification.user_question}
                    disabled={!localConfig.notification.enabled}
                    onCheckedChange={(checked) =>
                      updateConfig(["notification", "user_question"], checked)
                    }
                  />
                  <div>
                    <Label
                      htmlFor="userQuestion"
                      className={cn(
                        !localConfig.notification.enabled &&
                          "text-muted-foreground"
                      )}
                    >
                      User questions
                    </Label>
                    <p className="text-xs text-muted-foreground">
                      Notify when AI agent asks a question.
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-3">
                  <SimpleCheckbox
                    id="reviewReady"
                    checked={localConfig.notification.review_ready}
                    disabled={!localConfig.notification.enabled}
                    onCheckedChange={(checked) =>
                      updateConfig(["notification", "review_ready"], checked)
                    }
                  />
                  <div>
                    <Label
                      htmlFor="reviewReady"
                      className={cn(
                        !localConfig.notification.enabled &&
                          "text-muted-foreground"
                      )}
                    >
                      Review ready
                    </Label>
                    <p className="text-xs text-muted-foreground">
                      Notify when AI work is complete and ready for review.
                    </p>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Save Button */}
          <div className="flex justify-end">
            <Button onClick={handleSave} disabled={isSaving}>
              {isSaving ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : saveSuccess ? (
                <Check className="h-4 w-4" />
              ) : (
                <Save className="h-4 w-4" />
              )}
              {saveSuccess ? "Saved" : "Save Changes"}
            </Button>
          </div>
        </div>
      )}

      {activeTab === SettingsTab.Credentials && (
        <div className="space-y-6">
          {/* GitHub */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle>GitHub</CardTitle>
                  <CardDescription>
                    Required scopes: repo, read:user, workflow
                  </CardDescription>
                </div>
                {credentialsStatus?.github_configured ? (
                  <Check className="h-5 w-5 text-green-500" />
                ) : (
                  <X className="h-5 w-5 text-muted-foreground" />
                )}
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex gap-2">
                <Input
                  type="password"
                  placeholder={
                    credentialsStatus?.github_configured
                      ? "••••••••••••••••"
                      : "ghp_..."
                  }
                  value={githubToken}
                  onChange={(e) => setGithubTokenState(e.target.value)}
                />
                <Button
                  onClick={handleGithubSave}
                  disabled={!githubToken || credentialSaving === "github"}
                >
                  {credentialSaving === "github" ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    "Save"
                  )}
                </Button>
              </div>
              <a
                href="https://github.com/settings/tokens/new"
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-primary hover:underline"
              >
                Create token on GitHub
              </a>
            </CardContent>
          </Card>

          {/* GitLab */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle>GitLab</CardTitle>
                  <CardDescription>
                    Required scopes: api, read_user, read_repository,
                    write_repository
                  </CardDescription>
                </div>
                {credentialsStatus?.gitlab_configured ? (
                  <Check className="h-5 w-5 text-green-500" />
                ) : (
                  <X className="h-5 w-5 text-muted-foreground" />
                )}
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex gap-2">
                <Input
                  type="password"
                  placeholder={
                    credentialsStatus?.gitlab_configured
                      ? "••••••••••••••••"
                      : "glpat-..."
                  }
                  value={gitlabToken}
                  onChange={(e) => setGitlabTokenState(e.target.value)}
                />
                <Button
                  onClick={handleGitlabSave}
                  disabled={!gitlabToken || credentialSaving === "gitlab"}
                >
                  {credentialSaving === "gitlab" ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    "Save"
                  )}
                </Button>
              </div>
              <a
                href="https://gitlab.com/-/user_settings/personal_access_tokens"
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-primary hover:underline"
              >
                Create token on GitLab
              </a>
            </CardContent>
          </Card>

          {/* Bitbucket */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle>Bitbucket</CardTitle>
                  <CardDescription>
                    Required permissions: Repository Read/Write, Pull Requests
                    Read/Write
                  </CardDescription>
                </div>
                {credentialsStatus?.bitbucket_configured ? (
                  <Check className="h-5 w-5 text-green-500" />
                ) : (
                  <X className="h-5 w-5 text-muted-foreground" />
                )}
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Input
                  placeholder={
                    credentialsStatus?.bitbucket_configured
                      ? "••••••••••••••••"
                      : "Username"
                  }
                  value={bitbucketUsername}
                  onChange={(e) => setBitbucketUsername(e.target.value)}
                />
                <div className="flex gap-2">
                  <Input
                    type="password"
                    placeholder={
                      credentialsStatus?.bitbucket_configured
                        ? "••••••••••••••••"
                        : "App Password"
                    }
                    value={bitbucketPassword}
                    onChange={(e) => setBitbucketPassword(e.target.value)}
                  />
                  <Button
                    onClick={handleBitbucketSave}
                    disabled={
                      !bitbucketUsername ||
                      !bitbucketPassword ||
                      credentialSaving === "bitbucket"
                    }
                  >
                    {credentialSaving === "bitbucket" ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      "Save"
                    )}
                  </Button>
                </div>
              </div>
              <a
                href="https://bitbucket.org/account/settings/app-passwords/"
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-primary hover:underline"
              >
                Create app password on Bitbucket
              </a>
            </CardContent>
          </Card>
        </div>
      )}

      {activeTab === SettingsTab.License && (
        <div className="space-y-6">
          {/* License Status */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <Key className="h-5 w-5" />
                    License
                  </CardTitle>
                  <CardDescription>
                    DeliDev requires a license for full functionality. $4/month via Polar.sh
                  </CardDescription>
                </div>
                {licenseInfo && (
                  <span className={cn("text-sm font-medium", getLicenseStatusColor(licenseInfo.status))}>
                    {getLicenseStatusLabel(licenseInfo.status)}
                  </span>
                )}
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              {licenseError && (
                <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-3">
                  <p className="text-sm text-destructive">{licenseError}</p>
                </div>
              )}

              {licenseInfo?.status === LicenseStatus.Active ? (
                <div className="space-y-4">
                  {/* Active license info */}
                  <div className="grid gap-3 text-sm">
                    {licenseInfo.display_key && (
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">License Key:</span>
                        <span className="font-mono">{licenseInfo.display_key}</span>
                      </div>
                    )}
                    {licenseInfo.customer_email && (
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">Email:</span>
                        <span>{licenseInfo.customer_email}</span>
                      </div>
                    )}
                    {licenseInfo.customer_name && (
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">Name:</span>
                        <span>{licenseInfo.customer_name}</span>
                      </div>
                    )}
                    {licenseInfo.expires_at && (
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">Expires:</span>
                        <span>{new Date(licenseInfo.expires_at).toLocaleDateString()}</span>
                      </div>
                    )}
                    {licenseInfo.activation_limit && (
                      <div className="flex justify-between">
                        <span className="text-muted-foreground">Activations:</span>
                        <span>{licenseInfo.activations_used ?? 1} / {licenseInfo.activation_limit}</span>
                      </div>
                    )}
                  </div>

                  {/* Actions */}
                  <div className="flex gap-2 pt-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleValidateLicense}
                      disabled={licenseLoading}
                    >
                      {licenseLoading ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <RefreshCw className="h-4 w-4" />
                      )}
                      Refresh
                    </Button>
                    <Button
                      variant="destructive"
                      size="sm"
                      onClick={handleRemoveLicense}
                      disabled={licenseLoading}
                    >
                      {licenseLoading ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <Trash2 className="h-4 w-4" />
                      )}
                      Remove License
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="space-y-4">
                  {/* License activation form */}
                  <div className="space-y-2">
                    <Label htmlFor="licenseKey">License Key</Label>
                    <div className="flex gap-2">
                      <Input
                        id="licenseKey"
                        type="password"
                        placeholder="Enter your license key..."
                        value={licenseKey}
                        onChange={(e) => setLicenseKey(e.target.value)}
                      />
                      <Button
                        onClick={handleActivateLicense}
                        disabled={!licenseKey || licenseLoading}
                      >
                        {licenseLoading ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          "Activate"
                        )}
                      </Button>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      Enter the license key you received from Polar.sh
                    </p>
                  </div>

                  <div className="pt-2 border-t">
                    <p className="text-sm text-muted-foreground mb-2">
                      Don't have a license?
                    </p>
                    <a
                      href="https://deli.dev/checkout?productIds"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-sm text-primary hover:underline"
                    >
                      Purchase a license
                    </a>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
