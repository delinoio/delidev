import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "../ui/dialog";
import { Button } from "../ui/button";
import { Label } from "../ui/label";
import { FileAutocompleteTextarea } from "../ui/file-autocomplete-textarea";
import { Input } from "../ui/input";
import { Select } from "../ui/select";
import { SimpleCheckbox } from "../ui/checkbox";
import { useRepositoriesStore } from "../../stores/repositories";
import { useRepositoryGroupsStore } from "../../stores/repositoryGroups";
import { useWorkspacesStore } from "../../stores/workspaces";
import { useTasksStore } from "../../stores/tasks";
import { useConfigStore } from "../../stores/config";
import { AIAgentType } from "../../types";
import * as api from "../../api";
import { RepositoryGroupSelector } from "../repositoryGroups/RepositoryGroupSelector";

interface CreateTaskDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const agentTypeOptions = [
  { value: AIAgentType.ClaudeCode, label: "Claude Code" },
  { value: AIAgentType.OpenCode, label: "OpenCode" },
];

// localStorage keys for remembering last used values
const STORAGE_KEY_REPOSITORY_GROUP = "delidev:createTask:repositoryGroupId";
const STORAGE_KEY_COMPOSITE = "delidev:createTask:isComposite";

// Helper functions to safely access localStorage (can throw in private browsing mode or when storage is full)
const safeGetLocalStorage = (key: string): string | null => {
  try {
    return localStorage.getItem(key);
  } catch (error) {
    console.warn(`Failed to read from localStorage (key: ${key}):`, error);
    return null;
  }
};

const safeSetLocalStorage = (key: string, value: string): void => {
  try {
    localStorage.setItem(key, value);
  } catch (error) {
    console.warn(`Failed to write to localStorage (key: ${key}):`, error);
  }
};

export function CreateTaskDialog({ open, onOpenChange }: CreateTaskDialogProps) {
  const navigate = useNavigate();
  const [prompt, setPrompt] = useState("");
  const [title, setTitle] = useState("");
  const [branchName, setBranchName] = useState("");
  const [repositoryGroupId, setRepositoryGroupId] = useState("");
  const [isComposite, setIsComposite] = useState(() => {
    const saved = safeGetLocalStorage(STORAGE_KEY_COMPOSITE);
    return saved !== null ? saved === "true" : true;
  });
  // Agent type for UnitTask (simple mode)
  const [agentType, setAgentType] = useState<AIAgentType>(AIAgentType.ClaudeCode);
  // Agent types for CompositeTask (composite mode)
  const [planningAgentType, setPlanningAgentType] = useState<AIAgentType>(AIAgentType.ClaudeCode);
  const [executionAgentType, setExecutionAgentType] = useState<AIAgentType>(AIAgentType.ClaudeCode);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { repositories, fetchRepositories } = useRepositoriesStore();
  const { groups, fetchGroups } = useRepositoryGroupsStore();
  const { getDefaultWorkspace } = useWorkspacesStore();
  const { createUnitTask, createCompositeTask } = useTasksStore();
  const { globalConfig, fetchGlobalConfig } = useConfigStore();

  useEffect(() => {
    if (open) {
      fetchRepositories();
      fetchGroups();
      fetchGlobalConfig();
      getDefaultWorkspace().catch(console.error);
    }
  }, [open, fetchRepositories, fetchGroups, fetchGlobalConfig, getDefaultWorkspace]);

  useEffect(() => {
    if (groups.length > 0 && !repositoryGroupId) {
      // Try to restore saved repository group, fallback to first group
      const savedGroupId = safeGetLocalStorage(STORAGE_KEY_REPOSITORY_GROUP);
      const isValidSavedGroup = savedGroupId && groups.some(g => g.id === savedGroupId);
      setRepositoryGroupId(isValidSavedGroup ? savedGroupId : groups[0].id);
    }
  }, [groups, repositoryGroupId]);

  // Set default agent types from global config
  useEffect(() => {
    if (globalConfig?.agent?.execution?.type) {
      setAgentType(globalConfig.agent.execution.type);
      setExecutionAgentType(globalConfig.agent.execution.type);
    }
    if (globalConfig?.agent?.planning?.type) {
      setPlanningAgentType(globalConfig.agent.planning.type);
    }
  }, [globalConfig]);

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();

    if (!repositoryGroupId) return;
    if (!prompt) return;
    if (!title) return; // Title is now required

    // Save current selections to localStorage for next time
    safeSetLocalStorage(STORAGE_KEY_REPOSITORY_GROUP, repositoryGroupId);
    safeSetLocalStorage(STORAGE_KEY_COMPOSITE, String(isComposite));

    setIsSubmitting(true);
    try {
      if (isComposite) {
        const task = await createCompositeTask({
          repositoryGroupId,
          prompt,
          title,
          planningAgentType,
          executionAgentType,
        });

        // Check if Docker is available and start planning
        const dockerAvailable = await api.isDockerAvailable();
        if (dockerAvailable) {
          // Start planning in the background - don't await it
          api.startCompositeTaskPlanning(task.id).catch((planError) => {
            console.error("Failed to start planning:", planError);
            toast.error("Failed to start planning. Please try again manually.");
          });
          toast.success("Composite task created. Starting planning...");
        } else {
          toast.error("Docker/Podman is not available. Please start your container runtime to execute tasks.");
        }

        // Navigate to composite task detail page
        navigate(`/composite-tasks/${task.id}`);
      } else {
        // Create UnitTask and auto-execute if Docker is available
        // Use empty string for branch name if not provided (will use default branch name generation)
        const task = await createUnitTask({
          repositoryGroupId,
          prompt,
          title,
          branchName: branchName || "",
          agentType,
        });

        // Check if Docker is available and auto-execute
        const dockerAvailable = await api.isDockerAvailable();
        if (dockerAvailable) {
          try {
            await api.startTaskExecution(task.id);
            toast.success("Task created and execution started");
          } catch (execError) {
            console.error("Failed to start execution:", execError);
            toast.error("Task created but failed to start execution");
          }
        } else {
          toast.error("Docker/Podman is not available. Please start your container runtime to execute tasks.");
        }

        // Navigate to task detail page
        navigate(`/unit-tasks/${task.id}`);
      }
      onOpenChange(false);
      resetForm();
    } catch (error) {
      console.error("Failed to create task:", error);
      toast.error(error instanceof Error ? error.message : "Failed to create task");
    } finally {
      setIsSubmitting(false);
    }
  };

  const resetForm = () => {
    setPrompt("");
    setTitle("");
    setBranchName("");
    // Keep isComposite and repositoryGroupId as they're saved to localStorage
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Create Task</DialogTitle>
          <DialogDescription>
            Create a new task for AI agents to work on.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label>Repository Group</Label>
            <RepositoryGroupSelector
              groups={groups}
              repositories={repositories}
              selectedGroupId={repositoryGroupId}
              onSelectGroup={setRepositoryGroupId}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="prompt">Prompt</Label>
            <FileAutocompleteTextarea
              id="prompt"
              value={prompt}
              onValueChange={setPrompt}
              onKeyDown={handleKeyDown}
              repositoryGroupId={repositoryGroupId}
              placeholder="Describe the task in detail. Type @ to reference files..."
              rows={6}
              autoFocus
            />
            <p className="text-xs text-muted-foreground">
              Tip: Type @ to autocomplete and reference files in the repositories.
            </p>
          </div>

          <div className="space-y-4 rounded-lg border p-4">
            <Label className="text-sm font-medium">Title & Branch</Label>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="title" className="text-xs text-muted-foreground">Task Title (required)</Label>
                <Input
                  id="title"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="Enter a descriptive title"
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="branchName" className="text-xs text-muted-foreground">Branch Name (optional)</Label>
                <Input
                  id="branchName"
                  value={branchName}
                  onChange={(e) => setBranchName(e.target.value)}
                  placeholder="Leave empty for default"
                />
              </div>
            </div>
            <p className="text-xs text-muted-foreground">
              Provide a descriptive title for the task. Branch name is optional and will use a default pattern if not provided.
            </p>
          </div>

          {isComposite ? (
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="planningAgentType">Planning Agent</Label>
                <Select
                  id="planningAgentType"
                  options={agentTypeOptions}
                  value={planningAgentType}
                  onChange={(e) => setPlanningAgentType(e.target.value as AIAgentType)}
                />
                <p className="text-xs text-muted-foreground">
                  Agent for generating the task plan.
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="executionAgentType">Execution Agent</Label>
                <Select
                  id="executionAgentType"
                  options={agentTypeOptions}
                  value={executionAgentType}
                  onChange={(e) => setExecutionAgentType(e.target.value as AIAgentType)}
                />
                <p className="text-xs text-muted-foreground">
                  Agent for executing each sub-task.
                </p>
              </div>
            </div>
          ) : (
            <div className="space-y-2">
              <Label htmlFor="agentType">Agent</Label>
              <Select
                id="agentType"
                options={agentTypeOptions}
                value={agentType}
                onChange={(e) => setAgentType(e.target.value as AIAgentType)}
              />
              <p className="text-xs text-muted-foreground">
                The AI coding agent to use for this task.
              </p>
            </div>
          )}

          <div className="space-y-2">
            <div className="flex items-center gap-2 rounded-lg border p-4">
              <SimpleCheckbox
                id="composite"
                checked={isComposite}
                onCheckedChange={setIsComposite}
              />
              <div className="flex-1">
                <Label htmlFor="composite" className="cursor-pointer">
                  Composite mode
                </Label>
                <p className="text-xs text-muted-foreground">
                  Creates a CompositeTask with AI-generated plan. Uncheck for
                  simple single-step tasks (UnitTask).
                </p>
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={
                isSubmitting ||
                !prompt ||
                !title ||
                !repositoryGroupId
              }
            >
              {isSubmitting ? "Creating..." : "Create Task"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
