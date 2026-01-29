import { Link } from "react-router-dom";
import { Crown } from "lucide-react";
import { cn } from "../../lib/utils";

interface PremiumBadgeProps {
  className?: string;
}

/**
 * PremiumBadge component displays a badge indicating that a feature requires a premium license.
 * It includes a tooltip with more details and links to the license settings page.
 */
export function PremiumBadge({ className }: PremiumBadgeProps) {
  return (
    <Link
      to="/settings/license"
      className={cn(
        "group relative inline-flex items-center gap-1 px-1.5 py-0.5 text-xs font-medium rounded",
        "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/50 dark:text-yellow-200",
        "hover:bg-yellow-200 dark:hover:bg-yellow-900/70 transition-colors",
        "cursor-pointer",
        className
      )}
    >
      <Crown className="h-3 w-3" />
      <span>Premium</span>
      {/* Tooltip */}
      <div
        className={cn(
          "absolute left-1/2 -translate-x-1/2 bottom-full mb-2",
          "invisible group-hover:visible opacity-0 group-hover:opacity-100",
          "transition-opacity duration-200",
          "w-48 p-2 text-xs text-center",
          "bg-popover text-popover-foreground border rounded-md shadow-md",
          "z-50"
        )}
      >
        <p>This feature requires a premium license.</p>
        <p className="mt-1 text-muted-foreground">Click to configure license.</p>
        {/* Arrow */}
        <div
          className={cn(
            "absolute left-1/2 -translate-x-1/2 top-full",
            "border-4 border-transparent border-t-popover",
            "-mt-[1px]"
          )}
        />
      </div>
    </Link>
  );
}
