import { NavLink } from "react-router-dom";
import {
  LayoutDashboard,
  FolderGit2,
  Layers,
  Settings,
  MessageSquare,
  Plus,
} from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { WorkspaceSelector } from "../workspaces";

interface SidebarProps {
  onNewTask?: () => void;
}

export function Sidebar({ onNewTask }: SidebarProps) {
  const navItems = [
    { to: "/", icon: LayoutDashboard, label: "Dashboard" },
    { to: "/repositories", icon: FolderGit2, label: "Repositories" },
    { to: "/repository-groups", icon: Layers, label: "Repository Groups" },
    { to: "/settings", icon: Settings, label: "Settings" },
  ];

  return (
    <aside className="fixed left-0 top-0 z-40 h-screen w-64 border-r bg-card">
      <div className="flex h-full flex-col">
        {/* Logo */}
        <div className="flex h-16 items-center border-b px-6">
          <h1 className="text-xl font-bold">DeliDev</h1>
        </div>

        {/* Workspace Selector */}
        <div className="border-b px-4 py-3">
          <WorkspaceSelector />
        </div>

        {/* New Task Button */}
        <div className="p-4">
          <Button className="w-full" onClick={onNewTask}>
            <Plus className="h-4 w-4" />
            New Task
          </Button>
        </div>

        {/* Navigation */}
        <nav className="flex-1 space-y-1 px-3">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                  isActive
                    ? "bg-secondary text-secondary-foreground"
                    : "text-muted-foreground hover:bg-secondary hover:text-secondary-foreground"
                )
              }
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </NavLink>
          ))}
        </nav>

        {/* Chat Button */}
        <div className="border-t p-4">
          <NavLink
            to="/chat"
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "bg-secondary text-secondary-foreground hover:bg-secondary/80"
              )
            }
          >
            <MessageSquare className="h-4 w-4" />
            Chat
            <span className="ml-auto text-xs text-muted-foreground">
              Alt+Z
            </span>
          </NavLink>
        </div>
      </div>
    </aside>
  );
}
