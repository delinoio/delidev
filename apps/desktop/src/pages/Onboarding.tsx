import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { Select } from "../components/ui/select";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import { Loader2, Check, FolderGit2, ArrowRight, ArrowLeft } from "lucide-react";
import { VCSProviderType } from "../types";
import { useConfigStore } from "../stores/config";
import { useRepositoriesStore } from "../stores/repositories";

const vcsProviderOptions = [
  { value: VCSProviderType.GitHub, label: "GitHub" },
  { value: VCSProviderType.GitLab, label: "GitLab" },
  { value: VCSProviderType.Bitbucket, label: "Bitbucket" },
];

const providerHelpLinks: Record<VCSProviderType, string> = {
  [VCSProviderType.GitHub]: "https://github.com/settings/tokens/new",
  [VCSProviderType.GitLab]:
    "https://gitlab.com/-/user_settings/personal_access_tokens",
  [VCSProviderType.Bitbucket]:
    "https://bitbucket.org/account/settings/app-passwords/",
};

const providerScopes: Record<VCSProviderType, string> = {
  [VCSProviderType.GitHub]: "repo, read:user, workflow",
  [VCSProviderType.GitLab]: "api, read_user, read_repository, write_repository",
  [VCSProviderType.Bitbucket]:
    "Repository: Read/Write, Pull Requests: Read/Write",
};

export function Onboarding() {
  const navigate = useNavigate();
  const {
    setGithubToken,
    setGitlabToken,
    setBitbucketCredentials,
    fetchCredentialsStatus,
    credentialsStatus,
  } = useConfigStore();
  const { addRepository } = useRepositoriesStore();

  const [step, setStep] = useState(1);
  const [initialCheckDone, setInitialCheckDone] = useState(false);

  // Check if any provider credentials already exist on mount
  useEffect(() => {
    const checkCredentials = async () => {
      await fetchCredentialsStatus();
      setInitialCheckDone(true);
    };
    checkCredentials();
  }, [fetchCredentialsStatus]);

  // Skip to step 2 if any credentials are already configured
  useEffect(() => {
    if (initialCheckDone && credentialsStatus) {
      const hasAnyCredentials =
        credentialsStatus.github_configured ||
        credentialsStatus.gitlab_configured ||
        credentialsStatus.bitbucket_configured;
      if (hasAnyCredentials) {
        setStep(2);
      }
    }
  }, [initialCheckDone, credentialsStatus]);

  // Step 1 state
  const [selectedProvider, setSelectedProvider] = useState<VCSProviderType>(
    VCSProviderType.GitHub
  );
  const [token, setToken] = useState("");
  const [bitbucketUsername, setBitbucketUsername] = useState("");
  const [isValidating, setIsValidating] = useState(false);
  const [isValidated, setIsValidated] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [authenticatedUser, setAuthenticatedUser] = useState<string | null>(
    null
  );

  // Step 2 state
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [isAddingRepo, setIsAddingRepo] = useState(false);
  const [repoInfo, setRepoInfo] = useState<{
    name: string;
    remote: string;
    branch: string;
  } | null>(null);

  const handleValidateToken = async () => {
    if (!token) return;

    setIsValidating(true);
    setValidationError(null);

    try {
      let user;
      if (selectedProvider === VCSProviderType.GitHub) {
        user = await setGithubToken(token);
      } else if (selectedProvider === VCSProviderType.GitLab) {
        user = await setGitlabToken(token);
      } else {
        user = await setBitbucketCredentials(bitbucketUsername, token);
      }
      setAuthenticatedUser(user.username);
      setIsValidated(true);
    } catch (error) {
      setValidationError(
        error instanceof Error ? error.message : "Validation failed"
      );
    } finally {
      setIsValidating(false);
    }
  };

  const handleSelectRepository = async () => {
    try {
      const selected = await open({
        directory: true,
        title: "Select Repository Folder",
      });

      if (selected && typeof selected === "string") {
        setSelectedPath(selected);
        // Extract basic info
        const pathParts = selected.split("/");
        setRepoInfo({
          name: pathParts[pathParts.length - 1],
          remote: "Loading...",
          branch: "main",
        });
      }
    } catch (error) {
      console.error("Failed to select folder:", error);
    }
  };

  const handleAddRepository = async () => {
    if (!selectedPath) return;

    setIsAddingRepo(true);
    try {
      await addRepository(selectedPath);
      navigate("/");
    } catch (error) {
      console.error("Failed to add repository:", error);
    } finally {
      setIsAddingRepo(false);
    }
  };

  const handleSkip = () => {
    if (step === 1) {
      setStep(2);
    } else {
      navigate("/");
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <Card className="w-full max-w-lg">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">Welcome to DeliDev</CardTitle>
          <CardDescription>
            {step === 1 ? "Step 1 of 2" : "Step 2 of 2"}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {step === 1 && (
            <div className="space-y-6">
              <div>
                <h3 className="font-medium mb-2">Connect your VCS Provider</h3>
                <p className="text-sm text-muted-foreground">
                  Select a provider and enter your access token.
                </p>
              </div>

              <div className="space-y-4">
                <div className="space-y-2">
                  <Label>Provider</Label>
                  <Select
                    options={vcsProviderOptions}
                    value={selectedProvider}
                    onChange={(e) => {
                      setSelectedProvider(e.target.value as VCSProviderType);
                      setIsValidated(false);
                      setToken("");
                      setBitbucketUsername("");
                    }}
                  />
                </div>

                {selectedProvider === VCSProviderType.Bitbucket && (
                  <div className="space-y-2">
                    <Label>Username</Label>
                    <Input
                      value={bitbucketUsername}
                      onChange={(e) => setBitbucketUsername(e.target.value)}
                      placeholder="your-username"
                    />
                  </div>
                )}

                <div className="space-y-2">
                  <Label>
                    {selectedProvider === VCSProviderType.Bitbucket
                      ? "App Password"
                      : "Personal Access Token"}
                  </Label>
                  <Input
                    type="password"
                    value={token}
                    onChange={(e) => {
                      setToken(e.target.value);
                      setIsValidated(false);
                    }}
                    placeholder={
                      selectedProvider === VCSProviderType.GitHub
                        ? "ghp_..."
                        : selectedProvider === VCSProviderType.GitLab
                        ? "glpat-..."
                        : "App password"
                    }
                  />
                  <p className="text-xs text-muted-foreground">
                    Required scopes: {providerScopes[selectedProvider]}
                  </p>
                  <a
                    href={providerHelpLinks[selectedProvider]}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs text-primary hover:underline"
                  >
                    Create token on{" "}
                    {selectedProvider.charAt(0).toUpperCase() +
                      selectedProvider.slice(1)}
                  </a>
                </div>

                {!isValidated && (
                  <Button
                    className="w-full"
                    onClick={handleValidateToken}
                    disabled={
                      isValidating ||
                      !token ||
                      (selectedProvider === VCSProviderType.Bitbucket &&
                        !bitbucketUsername)
                    }
                  >
                    {isValidating ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      "Validate"
                    )}
                  </Button>
                )}

                {validationError && (
                  <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-3">
                    <p className="text-sm text-destructive">{validationError}</p>
                  </div>
                )}

                {isValidated && (
                  <div className="rounded-lg border border-green-500/50 bg-green-50 p-3 flex items-center gap-2">
                    <Check className="h-4 w-4 text-green-500" />
                    <div>
                      <p className="text-sm font-medium text-green-800">
                        Connection successful
                      </p>
                      <p className="text-xs text-green-600">
                        Authenticated as: @{authenticatedUser}
                      </p>
                    </div>
                  </div>
                )}
              </div>

              <div className="flex justify-between pt-4 border-t">
                <Button variant="ghost" onClick={handleSkip}>
                  Skip
                </Button>
                <Button onClick={() => setStep(2)} disabled={!isValidated}>
                  Next
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
          )}

          {step === 2 && (
            <div className="space-y-6">
              <div>
                <h3 className="font-medium mb-2">Add Your First Repository</h3>
                <p className="text-sm text-muted-foreground">
                  Select a local git repository to get started.
                </p>
              </div>

              <div
                className="border-2 border-dashed rounded-lg p-8 text-center cursor-pointer hover:border-primary/50 transition-colors"
                onClick={handleSelectRepository}
              >
                {selectedPath ? (
                  <div className="space-y-2">
                    <FolderGit2 className="h-12 w-12 mx-auto text-primary" />
                    <p className="font-medium">{repoInfo?.name}</p>
                    <p className="text-sm text-muted-foreground">
                      {selectedPath}
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    <FolderGit2 className="h-12 w-12 mx-auto text-muted-foreground" />
                    <p className="text-sm text-muted-foreground">
                      Click to select a repository folder
                    </p>
                  </div>
                )}
              </div>

              <p className="text-xs text-muted-foreground text-center">
                You can add more repositories later from Repository Management.
              </p>

              <div className="flex justify-between pt-4 border-t">
                <Button variant="ghost" onClick={() => setStep(1)}>
                  <ArrowLeft className="h-4 w-4" />
                  Back
                </Button>
                <div className="flex gap-2">
                  <Button variant="outline" onClick={handleSkip}>
                    Skip
                  </Button>
                  <Button
                    onClick={handleAddRepository}
                    disabled={!selectedPath || isAddingRepo}
                  >
                    {isAddingRepo ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      "Get Started"
                    )}
                  </Button>
                </div>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
