import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Button } from "../ui/button";
import { Popover, PopoverTrigger, PopoverContent } from "../ui/popover";
import {
  Command,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandSeparator,
} from "../ui/command";
import { Check, ChevronsUpDown, Layers, FolderGit2, Settings } from "lucide-react";
import { cn } from "../../lib/utils";
import type { RepositoryGroup, Repository } from "../../types";

interface RepositoryGroupSelectorProps {
  groups: RepositoryGroup[];
  repositories: Repository[];
  selectedGroupId: string;
  onSelectGroup: (groupId: string) => void;
  disabled?: boolean;
  placeholder?: string;
}

export function RepositoryGroupSelector({
  groups,
  repositories,
  selectedGroupId,
  onSelectGroup,
  disabled = false,
  placeholder = "Select a repository group",
}: RepositoryGroupSelectorProps) {
  const navigate = useNavigate();
  const [isPopoverOpen, setIsPopoverOpen] = useState(false);

  const selectedGroup = groups.find((g) => g.id === selectedGroupId);

  const getRepositoriesForGroup = (group: RepositoryGroup): Repository[] => {
    return group.repository_ids
      .map((id) => repositories.find((r) => r.id === id))
      .filter((r): r is Repository => r !== undefined);
  };

  const getGroupDisplayName = (group: RepositoryGroup): string => {
    if (group.name) {
      return group.name;
    }
    // For unnamed groups (single-repo), show repository name
    const repos = getRepositoriesForGroup(group);
    if (repos.length === 1) {
      return repos[0].name;
    }
    return `Group (${group.repository_ids.length} repos)`;
  };

  const handleSelectGroup = (groupId: string) => {
    onSelectGroup(groupId);
    setIsPopoverOpen(false);
  };

  const handleManageGroups = () => {
    setIsPopoverOpen(false);
    navigate("/repository-groups");
  };

  // Generate a consistent color based on group id
  const getGroupColor = (id: string) => {
    const colors = [
      "bg-blue-500/20 text-blue-500",
      "bg-green-500/20 text-green-500",
      "bg-purple-500/20 text-purple-500",
      "bg-orange-500/20 text-orange-500",
      "bg-pink-500/20 text-pink-500",
      "bg-cyan-500/20 text-cyan-500",
    ];
    let hash = 0;
    for (let i = 0; i < id.length; i++) {
      hash = id.charCodeAt(i) + ((hash << 5) - hash);
    }
    return colors[Math.abs(hash) % colors.length];
  };

  if (groups.length === 0) {
    return (
      <div className="space-y-2">
        <Button
          variant="outline"
          className="w-full justify-between h-9 px-3 font-normal text-muted-foreground"
          disabled
        >
          <span>No repository groups available</span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
        <p className="text-xs text-muted-foreground">
          <button
            type="button"
            onClick={() => navigate("/repository-groups")}
            className="text-primary hover:underline"
          >
            Create a repository group
          </button>
          {" "}to start creating tasks.
        </p>
      </div>
    );
  }

  const selectedRepos = selectedGroup ? getRepositoriesForGroup(selectedGroup) : [];

  return (
    <div className="space-y-2">
      <Popover open={isPopoverOpen} onOpenChange={setIsPopoverOpen}>
        <div className="relative">
          <PopoverTrigger asChild>
            <Button
              variant="outline"
              role="combobox"
              aria-expanded={isPopoverOpen}
              className="w-full justify-between h-9 px-3 font-normal"
              disabled={disabled}
            >
              <div className="flex items-center gap-2 truncate">
                {selectedGroup ? (
                  <>
                    <div
                      className={cn(
                        "flex h-5 w-5 shrink-0 items-center justify-center rounded",
                        getGroupColor(selectedGroup.id)
                      )}
                    >
                      <Layers className="h-3 w-3" />
                    </div>
                    <span className="truncate">{getGroupDisplayName(selectedGroup)}</span>
                    <span className="text-xs text-muted-foreground">
                      ({selectedGroup.repository_ids.length})
                    </span>
                  </>
                ) : (
                  <span className="text-muted-foreground">{placeholder}</span>
                )}
              </div>
              <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
            </Button>
          </PopoverTrigger>

          <PopoverContent className="w-[var(--radix-popover-trigger-width)] p-0" align="start">
            <Command>
              <CommandInput placeholder="Search groups..." />
              <CommandList>
                <CommandEmpty>No group found.</CommandEmpty>
                <CommandGroup>
                  {groups.map((group) => {
                    const repos = getRepositoriesForGroup(group);
                    const displayName = getGroupDisplayName(group);
                    return (
                      <CommandItem
                        key={group.id}
                        value={displayName}
                        onSelect={() => handleSelectGroup(group.id)}
                        className="flex flex-col items-start gap-1 py-2"
                      >
                        <div className="flex items-center w-full">
                          <div
                            className={cn(
                              "mr-2 flex h-5 w-5 shrink-0 items-center justify-center rounded",
                              getGroupColor(group.id)
                            )}
                          >
                            <Layers className="h-3 w-3" />
                          </div>
                          <span className="flex-1 truncate font-medium">{displayName}</span>
                          <span className="text-xs text-muted-foreground mr-2">
                            {repos.length} {repos.length === 1 ? "repo" : "repos"}
                          </span>
                          {group.id === selectedGroupId && (
                            <Check className="h-4 w-4 shrink-0 text-primary" />
                          )}
                        </div>
                        {repos.length > 0 && (
                          <div className="flex flex-wrap gap-1 ml-7 w-full">
                            {repos.slice(0, 4).map((repo) => (
                              <span
                                key={repo.id}
                                className="inline-flex items-center gap-1 px-1.5 py-0.5 text-[10px] rounded bg-muted text-muted-foreground"
                              >
                                <FolderGit2 className="h-2.5 w-2.5" />
                                {repo.name}
                              </span>
                            ))}
                            {repos.length > 4 && (
                              <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] rounded bg-muted text-muted-foreground">
                                +{repos.length - 4} more
                              </span>
                            )}
                          </div>
                        )}
                      </CommandItem>
                    );
                  })}
                </CommandGroup>
                <CommandSeparator />
                <CommandGroup>
                  <CommandItem onSelect={handleManageGroups}>
                    <Settings className="mr-2 h-4 w-4" />
                    <span>Manage repository groups</span>
                  </CommandItem>
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </div>
      </Popover>

      {selectedRepos.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {selectedRepos.map((repo) => (
            <div
              key={repo.id}
              className="flex items-center gap-1.5 px-2 py-1 text-xs rounded-md bg-muted"
            >
              <FolderGit2 className="h-3 w-3" />
              <span>{repo.name}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
