import { useEffect, useState } from "react";
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
import { useWorkspacesStore } from "../../stores/workspaces";
import { Briefcase, Check, ChevronsUpDown, Plus } from "lucide-react";
import { CreateWorkspaceDialog } from "./CreateWorkspaceDialog";
import { cn } from "../../lib/utils";

export function WorkspaceSelector() {
  const {
    workspaces,
    selectedWorkspaceId,
    fetchWorkspaces,
    selectWorkspace,
    getDefaultWorkspace,
    hasFetched,
  } = useWorkspacesStore();

  const [isPopoverOpen, setIsPopoverOpen] = useState(false);
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);

  useEffect(() => {
    if (!hasFetched) {
      fetchWorkspaces();
    }
  }, [hasFetched, fetchWorkspaces]);

  // Ensure a workspace is selected
  useEffect(() => {
    if (hasFetched && !selectedWorkspaceId && workspaces.length === 0) {
      // No workspaces exist, create default
      getDefaultWorkspace().catch(console.error);
    } else if (hasFetched && !selectedWorkspaceId && workspaces.length > 0) {
      // Select first workspace if none selected
      selectWorkspace(workspaces[0].id);
    }
  }, [hasFetched, selectedWorkspaceId, workspaces, getDefaultWorkspace, selectWorkspace]);

  const selectedWorkspace = workspaces.find((ws) => ws.id === selectedWorkspaceId);

  if (workspaces.length === 0) {
    return null;
  }

  const handleSelectWorkspace = (workspaceId: string) => {
    selectWorkspace(workspaceId);
    setIsPopoverOpen(false);
  };

  const handleCreateWorkspace = () => {
    setIsPopoverOpen(false);
    setIsCreateDialogOpen(true);
  };

  // Generate initials from workspace name
  const getInitials = (name: string) => {
    return name
      .split(/\s+/)
      .map((word) => word[0])
      .join("")
      .toUpperCase()
      .slice(0, 2);
  };

  // Generate a consistent color based on workspace id
  const getWorkspaceColor = (id: string) => {
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

  return (
    <>
      <div className="space-y-1.5">
        <label className="flex items-center gap-2 text-xs font-medium text-muted-foreground px-1">
          <Briefcase className="h-3 w-3" />
          Workspace
        </label>

        <Popover open={isPopoverOpen} onOpenChange={setIsPopoverOpen}>
          <div className="relative">
            <PopoverTrigger asChild>
              <Button
                variant="outline"
                role="combobox"
                aria-expanded={isPopoverOpen}
                className="w-full justify-between h-9 px-3 font-normal"
              >
                <div className="flex items-center gap-2 truncate">
                  {selectedWorkspace && (
                    <div
                      className={cn(
                        "flex h-5 w-5 shrink-0 items-center justify-center rounded text-[10px] font-semibold",
                        getWorkspaceColor(selectedWorkspace.id)
                      )}
                    >
                      {getInitials(selectedWorkspace.name)}
                    </div>
                  )}
                  <span className="truncate">
                    {selectedWorkspace?.name ?? "Select workspace..."}
                  </span>
                </div>
                <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
              </Button>
            </PopoverTrigger>

            <PopoverContent className="w-[var(--radix-popover-trigger-width)] p-0" align="start">
              <Command>
                <CommandInput placeholder="Search workspaces..." />
                <CommandList>
                  <CommandEmpty>No workspace found.</CommandEmpty>
                  <CommandGroup>
                    {workspaces.map((workspace) => (
                      <CommandItem
                        key={workspace.id}
                        value={workspace.name}
                        onSelect={() => handleSelectWorkspace(workspace.id)}
                      >
                        <div
                          className={cn(
                            "mr-2 flex h-5 w-5 shrink-0 items-center justify-center rounded text-[10px] font-semibold",
                            getWorkspaceColor(workspace.id)
                          )}
                        >
                          {getInitials(workspace.name)}
                        </div>
                        <span className="flex-1 truncate">{workspace.name}</span>
                        {workspace.id === selectedWorkspaceId && (
                          <Check className="ml-2 h-4 w-4 shrink-0 text-primary" />
                        )}
                      </CommandItem>
                    ))}
                  </CommandGroup>
                  <CommandSeparator />
                  <CommandGroup>
                    <CommandItem onSelect={handleCreateWorkspace}>
                      <Plus className="mr-2 h-4 w-4" />
                      <span>Create new workspace</span>
                    </CommandItem>
                  </CommandGroup>
                </CommandList>
              </Command>
            </PopoverContent>
          </div>
        </Popover>
      </div>

      <CreateWorkspaceDialog
        open={isCreateDialogOpen}
        onOpenChange={setIsCreateDialogOpen}
      />
    </>
  );
}
