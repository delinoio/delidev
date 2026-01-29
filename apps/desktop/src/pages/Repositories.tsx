import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useRepositoriesStore } from "../stores/repositories";
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
  FolderGit2,
  Plus,
  Settings,
  Trash2,
  Loader2,
  ExternalLink,
} from "lucide-react";
import { Link } from "react-router-dom";
import type { Repository } from "../types";
import { VCSProviderType } from "../types";

function getVcsProviderDisplayName(provider: VCSProviderType): string {
  switch (provider) {
    case VCSProviderType.GitHub:
      return "GitHub";
    case VCSProviderType.GitLab:
      return "GitLab";
    case VCSProviderType.Bitbucket:
      return "Bitbucket";
    default:
      return provider;
  }
}

export function Repositories() {
  const {
    repositories,
    fetchRepositories,
    addRepository,
    removeRepository,
    isLoading,
    error,
  } = useRepositoriesStore();

  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [repositoryToDelete, setRepositoryToDelete] = useState<Repository | null>(null);
  const [isAdding, setIsAdding] = useState(false);

  useEffect(() => {
    fetchRepositories();
  }, [fetchRepositories]);

  const handleAddRepository = async () => {
    try {
      setIsAdding(true);
      const selected = await open({
        directory: true,
        multiple: true,
        title: "Select Repository Folders",
      });

      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        for (const path of paths) {
          await addRepository(path);
        }
      }
    } catch (error) {
      console.error("Failed to add repository:", error);
    } finally {
      setIsAdding(false);
    }
  };

  const handleDeleteClick = (repo: Repository) => {
    setRepositoryToDelete(repo);
    setDeleteDialogOpen(true);
  };

  const handleConfirmDelete = async () => {
    if (repositoryToDelete) {
      await removeRepository(repositoryToDelete.id);
      setDeleteDialogOpen(false);
      setRepositoryToDelete(null);
    }
  };

  if (isLoading && repositories.length === 0) {
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
          <h1 className="text-2xl font-bold">Repositories</h1>
          <p className="text-muted-foreground">
            Manage your registered git repositories.
          </p>
        </div>
        <Button onClick={handleAddRepository} disabled={isAdding}>
          {isAdding ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Plus className="h-4 w-4" />
          )}
          Add Repositories
        </Button>
      </div>

      {error && (
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}

      {repositories.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <FolderGit2 className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-medium mb-2">No repositories</h3>
            <p className="text-sm text-muted-foreground mb-4">
              Add your first repository to get started.
            </p>
            <Button onClick={handleAddRepository}>
              <Plus className="h-4 w-4" />
              Add Repository
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {repositories.map((repo) => (
            <Card key={repo.id}>
              <CardHeader className="pb-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <FolderGit2 className="h-5 w-5 text-muted-foreground" />
                    <div>
                      <CardTitle className="text-lg">{repo.name}</CardTitle>
                      <CardDescription>{repo.local_path}</CardDescription>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Link to={`/settings/repository/${repo.id}`}>
                      <Button variant="ghost" size="icon">
                        <Settings className="h-4 w-4" />
                      </Button>
                    </Link>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDeleteClick(repo)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                <div className="flex items-center gap-4 text-sm text-muted-foreground">
                  <Badge variant="outline">{getVcsProviderDisplayName(repo.vcs_provider_type)}</Badge>
                  <span className="flex items-center gap-1">
                    Branch: {repo.default_branch}
                  </span>
                  {repo.remote_url && (
                    <a
                      href={repo.remote_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-1 hover:text-foreground"
                    >
                      <ExternalLink className="h-3 w-3" />
                      Remote
                    </a>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Remove Repository</DialogTitle>
            <DialogDescription>
              Are you sure you want to remove "{repositoryToDelete?.name}" from
              DeliDev? This will not delete the repository from your computer.
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
              Remove
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
