import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTabParams } from "../hooks";
import { useRepositoriesStore } from "../stores/repositories";
import * as api from "../api";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { Select } from "../components/ui/select";
import { SimpleCheckbox } from "../components/ui/checkbox";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import { Loader2, Save, Check, ArrowLeft } from "lucide-react";
import { AutoFixReviewFilter, type RepositoryConfig } from "../types";

const autoFixFilterOptions = [
  { value: AutoFixReviewFilter.WriteAccessOnly, label: "Write Access Only" },
  { value: AutoFixReviewFilter.All, label: "All" },
];

export function RepositorySettings() {
  const { id } = useTabParams<{ id: string }>();
  const { repositories, fetchRepositories } = useRepositoriesStore();

  const [localConfig, setLocalConfig] = useState<RepositoryConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const repository = repositories.find((r) => r.id === id);

  useEffect(() => {
    fetchRepositories();
  }, [fetchRepositories]);

  useEffect(() => {
    async function loadConfig() {
      if (!repository) return;

      try {
        setIsLoading(true);
        setError(null);
        const config = await api.getRepositoryConfig(repository.local_path);
        setLocalConfig(config);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load config");
      } finally {
        setIsLoading(false);
      }
    }

    loadConfig();
  }, [repository]);

  const handleSave = async () => {
    if (!localConfig || !repository) return;

    setIsSaving(true);
    try {
      await api.updateRepositoryConfig(repository.local_path, localConfig);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save config");
    } finally {
      setIsSaving(false);
    }
  };

  const updateConfig = (path: string[], value: unknown) => {
    if (!localConfig || path.length === 0) return;

    try {
      const newConfig = JSON.parse(JSON.stringify(localConfig)) as RepositoryConfig;
      let current: Record<string, unknown> = newConfig as unknown as Record<string, unknown>;

      for (let i = 0; i < path.length - 1; i++) {
        const key = path[i];
        if (typeof current[key] !== "object" || current[key] === null) {
          console.error(`Invalid config path: ${path.slice(0, i + 1).join(".")} is not an object`);
          return;
        }
        current = current[key] as Record<string, unknown>;
      }

      const lastKey = path[path.length - 1];
      current[lastKey] = value;
      setLocalConfig(newConfig);
    } catch (error) {
      console.error("Failed to update config:", error);
    }
  };

  if (isLoading || !repository) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Link to="/repositories">
          <Button variant="ghost" size="icon">
            <ArrowLeft className="h-4 w-4" />
          </Button>
        </Link>
        <div>
          <h1 className="text-2xl font-bold">Workspace Settings</h1>
          <p className="text-muted-foreground">
            Settings for {repository.name} ({repository.local_path})
          </p>
        </div>
      </div>

      {error && (
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {localConfig && (
        <div className="space-y-6">
          {/* Docker Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Docker</CardTitle>
              <CardDescription>
                Configure the Docker environment for AI agent execution.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <p className="text-sm">
                  Docker image configuration is done via{" "}
                  <code className="px-1 py-0.5 bg-muted rounded text-xs">
                    .delidev/setup/Dockerfile
                  </code>
                </p>
                <p className="text-xs text-muted-foreground">
                  If no Dockerfile is present, the default image (node:20-slim)
                  will be used. Create a Dockerfile in the .delidev/setup/
                  directory to customize the agent environment.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Branch Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Branch</CardTitle>
              <CardDescription>
                Configure branch naming for tasks.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                <Label htmlFor="branchTemplate">Branch Template</Label>
                <Input
                  id="branchTemplate"
                  value={localConfig.branch.template}
                  onChange={(e) =>
                    updateConfig(["branch", "template"], e.target.value)
                  }
                  placeholder="delidev/{task_id}"
                />
                <p className="text-xs text-muted-foreground">
                  Template for branch names. Use {"{task_id}"} as placeholder.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Automation Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Automation</CardTitle>
              <CardDescription>
                Configure automatic fixing of issues.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center gap-3">
                <SimpleCheckbox
                  id="autoFixReviews"
                  checked={localConfig.automation.auto_fix_review_comments}
                  onCheckedChange={(checked) =>
                    updateConfig(
                      ["automation", "auto_fix_review_comments"],
                      checked
                    )
                  }
                />
                <div>
                  <Label htmlFor="autoFixReviews">
                    Auto-fix review comments
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Automatically apply review comments from VCS provider.
                  </p>
                </div>
              </div>

              {localConfig.automation.auto_fix_review_comments && (
                <div className="ml-6 space-y-2">
                  <Label>Review Comment Filter</Label>
                  <Select
                    options={autoFixFilterOptions}
                    value={
                      localConfig.automation.auto_fix_review_comments_filter
                    }
                    onChange={(e) =>
                      updateConfig(
                        ["automation", "auto_fix_review_comments_filter"],
                        e.target.value
                      )
                    }
                  />
                  <p className="text-xs text-muted-foreground">
                    Which review comments to auto-fix.
                  </p>
                </div>
              )}

              <div className="flex items-center gap-3">
                <SimpleCheckbox
                  id="autoFixCi"
                  checked={localConfig.automation.auto_fix_ci_failures}
                  onCheckedChange={(checked) =>
                    updateConfig(
                      ["automation", "auto_fix_ci_failures"],
                      checked
                    )
                  }
                />
                <div>
                  <Label htmlFor="autoFixCi">Auto-fix CI failures</Label>
                  <p className="text-xs text-muted-foreground">
                    Automatically fix CI failures.
                  </p>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="maxAttempts">Max auto-fix attempts</Label>
                <Input
                  id="maxAttempts"
                  type="number"
                  min={1}
                  max={10}
                  value={localConfig.automation.max_auto_fix_attempts}
                  onChange={(e) =>
                    updateConfig(
                      ["automation", "max_auto_fix_attempts"],
                      parseInt(e.target.value, 10) || 1
                    )
                  }
                />
                <p className="text-xs text-muted-foreground">
                  Maximum number of auto-fix attempts before manual
                  intervention.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Learning Settings (Override) */}
          <Card>
            <CardHeader>
              <CardTitle>Learning (Override)</CardTitle>
              <CardDescription>
                Override global learning settings for this repository.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                <div className="flex items-center gap-3">
                  <input
                    type="radio"
                    id="learning-global"
                    name="learning"
                    checked={
                      localConfig.learning.auto_learn_from_reviews === undefined
                    }
                    onChange={() =>
                      updateConfig(
                        ["learning", "auto_learn_from_reviews"],
                        undefined
                      )
                    }
                    className="h-4 w-4"
                  />
                  <Label htmlFor="learning-global">Use global setting</Label>
                </div>
                <div className="flex items-center gap-3">
                  <input
                    type="radio"
                    id="learning-on"
                    name="learning"
                    checked={
                      localConfig.learning.auto_learn_from_reviews === true
                    }
                    onChange={() =>
                      updateConfig(["learning", "auto_learn_from_reviews"], true)
                    }
                    className="h-4 w-4"
                  />
                  <Label htmlFor="learning-on">Override: On</Label>
                </div>
                <div className="flex items-center gap-3">
                  <input
                    type="radio"
                    id="learning-off"
                    name="learning"
                    checked={
                      localConfig.learning.auto_learn_from_reviews === false
                    }
                    onChange={() =>
                      updateConfig(
                        ["learning", "auto_learn_from_reviews"],
                        false
                      )
                    }
                    className="h-4 w-4"
                  />
                  <Label htmlFor="learning-off">Override: Off</Label>
                </div>
                <p className="text-xs text-muted-foreground">
                  Override the global auto-learn from reviews setting for this
                  repository.
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
    </div>
  );
}
