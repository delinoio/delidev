import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "../ui/dialog";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { SimpleCheckbox } from "../ui/checkbox";
import { FolderGit2 } from "lucide-react";
import { useRepositoryGroupsStore } from "../../stores/repositoryGroups";
import { useRepositoriesStore } from "../../stores/repositories";
import { useWorkspacesStore } from "../../stores/workspaces";
import { toast } from "sonner";

interface CreateRepositoryGroupDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CreateRepositoryGroupDialog({
  open,
  onOpenChange,
}: CreateRepositoryGroupDialogProps) {
  const [name, setName] = useState("");
  const [selectedRepoIds, setSelectedRepoIds] = useState<Set<string>>(new Set());
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { createGroup } = useRepositoryGroupsStore();
  const { repositories } = useRepositoriesStore();
  const { selectedWorkspaceId } = useWorkspacesStore();

  const handleToggleRepository = (repoId: string) => {
    setSelectedRepoIds((prev) => {
      const next = new Set(prev);
      if (next.has(repoId)) {
        next.delete(repoId);
      } else {
        next.add(repoId);
      }
      return next;
    });
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !selectedWorkspaceId) return;

    setIsSubmitting(true);
    try {
      await createGroup(
        selectedWorkspaceId,
        name.trim(),
        Array.from(selectedRepoIds)
      );
      toast.success("Repository group created");
      onOpenChange(false);
      resetForm();
    } catch (error) {
      console.error("Failed to create repository group:", error);
      toast.error(error instanceof Error ? error.message : "Failed to create repository group");
    } finally {
      setIsSubmitting(false);
    }
  };

  const resetForm = () => {
    setName("");
    setSelectedRepoIds(new Set());
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Create Repository Group</DialogTitle>
          <DialogDescription>
            Create a group of repositories for multi-repository tasks.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="name">Group Name</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My Repository Group"
              autoFocus
            />
          </div>

          <div className="space-y-2">
            <Label>Repositories</Label>
            <div className="border rounded-lg max-h-48 overflow-y-auto">
              {repositories.length === 0 ? (
                <p className="text-sm text-muted-foreground text-center py-4">
                  No repositories available. Add repositories first.
                </p>
              ) : (
                <div className="p-2 space-y-1">
                  {repositories.map((repo) => (
                    <label
                      key={repo.id}
                      className="flex items-center gap-3 p-2 rounded-lg hover:bg-muted cursor-pointer"
                    >
                      <SimpleCheckbox
                        checked={selectedRepoIds.has(repo.id)}
                        onCheckedChange={() => handleToggleRepository(repo.id)}
                      />
                      <FolderGit2 className="h-4 w-4 text-muted-foreground shrink-0" />
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate">{repo.name}</p>
                        <p className="text-xs text-muted-foreground truncate">
                          {repo.local_path}
                        </p>
                      </div>
                    </label>
                  ))}
                </div>
              )}
            </div>
            <p className="text-xs text-muted-foreground">
              {selectedRepoIds.size} {selectedRepoIds.size === 1 ? "repository" : "repositories"} selected
            </p>
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
              disabled={isSubmitting || !name.trim() || selectedRepoIds.size === 0}
            >
              {isSubmitting ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
