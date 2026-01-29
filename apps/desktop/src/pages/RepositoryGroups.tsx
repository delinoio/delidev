import { useEffect, useState } from "react";
import { useRepositoryGroupsStore } from "../stores/repositoryGroups";
import { useRepositoriesStore } from "../stores/repositories";
import { useWorkspacesStore } from "../stores/workspaces";
import { Button } from "../components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../components/ui/dialog";
import {
  Layers,
  Plus,
  Trash2,
  Loader2,
  FolderGit2,
  Edit2,
  X,
} from "lucide-react";
import { Input } from "../components/ui/input";
import { Label } from "../components/ui/label";
import { SimpleCheckbox } from "../components/ui/checkbox";
import type { RepositoryGroup, Repository } from "../types";
import { toast } from "sonner";
import { CreateRepositoryGroupDialog } from "../components/repositoryGroups/CreateRepositoryGroupDialog";

export function RepositoryGroups() {
  const {
    groups,
    fetchGroups,
    deleteGroup,
    updateGroup,
    addRepositoryToGroup,
    removeRepositoryFromGroup,
    isLoading,
    error,
  } = useRepositoryGroupsStore();

  const { repositories, fetchRepositories, hasFetched: reposHasFetched } = useRepositoriesStore();
  const { selectedWorkspaceId, hasFetched: workspacesHasFetched } = useWorkspacesStore();

  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [groupToDelete, setGroupToDelete] = useState<RepositoryGroup | null>(null);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);
  const [groupToEdit, setGroupToEdit] = useState<RepositoryGroup | null>(null);
  const [manageReposDialogOpen, setManageReposDialogOpen] = useState(false);
  const [groupToManageRepos, setGroupToManageRepos] = useState<RepositoryGroup | null>(null);

  useEffect(() => {
    if (!reposHasFetched) {
      fetchRepositories();
    }
  }, [reposHasFetched, fetchRepositories]);

  useEffect(() => {
    if (workspacesHasFetched && selectedWorkspaceId) {
      fetchGroups(selectedWorkspaceId);
    }
  }, [workspacesHasFetched, selectedWorkspaceId, fetchGroups]);

  // Filter groups: only show multi-repo groups (named groups)
  // Single-repo groups (name is null) are created automatically and don't need management
  const multiRepoGroups = groups.filter((g) => g.name !== null && g.name !== undefined);

  const getRepositoriesForGroup = (group: RepositoryGroup): Repository[] => {
    return repositories.filter((r) => group.repository_ids.includes(r.id));
  };

  const handleDeleteClick = (group: RepositoryGroup) => {
    setGroupToDelete(group);
    setDeleteDialogOpen(true);
  };

  const handleConfirmDelete = async () => {
    if (groupToDelete) {
      try {
        await deleteGroup(groupToDelete.id);
        toast.success("Repository group deleted");
        setDeleteDialogOpen(false);
        setGroupToDelete(null);
      } catch (error) {
        toast.error(error instanceof Error ? error.message : "Failed to delete group");
      }
    }
  };

  const handleEditClick = (group: RepositoryGroup) => {
    setGroupToEdit(group);
    setEditDialogOpen(true);
  };

  const handleManageReposClick = (group: RepositoryGroup) => {
    setGroupToManageRepos(group);
    setManageReposDialogOpen(true);
  };

  if (isLoading && groups.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Repository Groups</h1>
          <p className="text-muted-foreground">
            Create groups of repositories for multi-repository tasks.
          </p>
        </div>
        <Button onClick={() => setCreateDialogOpen(true)}>
          <Plus className="h-4 w-4" />
          Create Group
        </Button>
      </div>

      {error && (
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {multiRepoGroups.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Layers className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">No repository groups</h3>
            <p className="text-sm text-muted-foreground mb-4 text-center max-w-md">
              Repository groups allow you to run tasks across multiple repositories
              simultaneously. Create a group to get started.
            </p>
            <Button onClick={() => setCreateDialogOpen(true)}>
              <Plus className="h-4 w-4" />
              Create Group
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {multiRepoGroups.map((group) => (
            <RepositoryGroupCard
              key={group.id}
              group={group}
              repositories={getRepositoriesForGroup(group)}
              onDelete={() => handleDeleteClick(group)}
              onEdit={() => handleEditClick(group)}
              onManageRepos={() => handleManageReposClick(group)}
            />
          ))}
        </div>
      )}

      {/* Create Group Dialog */}
      <CreateRepositoryGroupDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
      />

      {/* Edit Group Dialog */}
      <EditRepositoryGroupDialog
        open={editDialogOpen}
        onOpenChange={setEditDialogOpen}
        group={groupToEdit}
        onSave={async (name) => {
          if (groupToEdit) {
            await updateGroup(groupToEdit.id, name);
            toast.success("Group updated");
          }
        }}
      />

      {/* Manage Repositories Dialog */}
      <ManageRepositoriesDialog
        open={manageReposDialogOpen}
        onOpenChange={setManageReposDialogOpen}
        group={groupToManageRepos}
        allRepositories={repositories}
        onAddRepository={addRepositoryToGroup}
        onRemoveRepository={removeRepositoryFromGroup}
      />

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Repository Group</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete "{groupToDelete?.name}"?
              This will not delete the repositories themselves, only the group.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDeleteDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleConfirmDelete}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

interface RepositoryGroupCardProps {
  group: RepositoryGroup;
  repositories: Repository[];
  onDelete: () => void;
  onEdit: () => void;
  onManageRepos: () => void;
}

function RepositoryGroupCard({
  group,
  repositories,
  onDelete,
  onEdit,
  onManageRepos,
}: RepositoryGroupCardProps) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Layers className="h-5 w-5 text-muted-foreground" />
            <div>
              <CardTitle className="text-lg">{group.name}</CardTitle>
              <CardDescription>
                {repositories.length} {repositories.length === 1 ? "repository" : "repositories"}
              </CardDescription>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={onEdit}
              title="Edit group name"
            >
              <Edit2 className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={onDelete}
              title="Delete group"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          <div className="flex flex-wrap gap-2">
            {repositories.length === 0 ? (
              <p className="text-sm text-muted-foreground">No repositories in this group</p>
            ) : (
              repositories.map((repo) => (
                <Badge key={repo.id} variant="secondary" className="flex items-center gap-1">
                  <FolderGit2 className="h-3 w-3" />
                  {repo.name}
                </Badge>
              ))
            )}
          </div>
          <Button variant="outline" size="sm" onClick={onManageRepos}>
            <Plus className="h-3 w-3" />
            Manage Repositories
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

interface EditRepositoryGroupDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  group: RepositoryGroup | null;
  onSave: (name: string) => Promise<void>;
}

function EditRepositoryGroupDialog({
  open,
  onOpenChange,
  group,
  onSave,
}: EditRepositoryGroupDialogProps) {
  const [name, setName] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (group) {
      setName(group.name ?? "");
    }
  }, [group]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setIsSubmitting(true);
    try {
      await onSave(name.trim());
      onOpenChange(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to update group");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Edit Repository Group</DialogTitle>
          <DialogDescription>
            Update the name of this repository group.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="edit-name">Name</Label>
            <Input
              id="edit-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My Group"
              autoFocus
            />
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting || !name.trim()}>
              {isSubmitting ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface ManageRepositoriesDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  group: RepositoryGroup | null;
  allRepositories: Repository[];
  onAddRepository: (groupId: string, repositoryId: string) => Promise<void>;
  onRemoveRepository: (groupId: string, repositoryId: string) => Promise<void>;
}

function ManageRepositoriesDialog({
  open,
  onOpenChange,
  group,
  allRepositories,
  onAddRepository,
  onRemoveRepository,
}: ManageRepositoriesDialogProps) {
  const [selectedRepoIds, setSelectedRepoIds] = useState<Set<string>>(new Set());
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (group) {
      setSelectedRepoIds(new Set(group.repository_ids));
    }
  }, [group]);

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

  const handleSave = async () => {
    if (!group) return;

    setIsSubmitting(true);
    try {
      const currentIds = new Set(group.repository_ids);

      // Add new repositories
      for (const repoId of selectedRepoIds) {
        if (!currentIds.has(repoId)) {
          await onAddRepository(group.id, repoId);
        }
      }

      // Remove unchecked repositories
      for (const repoId of currentIds) {
        if (!selectedRepoIds.has(repoId)) {
          await onRemoveRepository(group.id, repoId);
        }
      }

      toast.success("Repositories updated");
      onOpenChange(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to update repositories");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Manage Repositories</DialogTitle>
          <DialogDescription>
            Select which repositories should be included in "{group?.name}".
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-2 max-h-64 overflow-y-auto">
          {allRepositories.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-4">
              No repositories available. Add repositories first.
            </p>
          ) : (
            allRepositories.map((repo) => (
              <label
                key={repo.id}
                className="flex items-center gap-3 p-2 rounded-lg hover:bg-muted cursor-pointer"
              >
                <SimpleCheckbox
                  checked={selectedRepoIds.has(repo.id)}
                  onCheckedChange={() => handleToggleRepository(repo.id)}
                />
                <FolderGit2 className="h-4 w-4 text-muted-foreground" />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{repo.name}</p>
                  <p className="text-xs text-muted-foreground truncate">
                    {repo.local_path}
                  </p>
                </div>
              </label>
            ))
          )}
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
          >
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={isSubmitting}>
            {isSubmitting ? "Saving..." : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
