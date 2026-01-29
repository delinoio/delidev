import { useState, useCallback, useMemo } from "react";
import { FileDiff, type FileDiffMetadata } from "@pierre/diffs/react";
import { ChevronRight, ChevronDown, Plus, Minus, File } from "lucide-react";

interface LazyFileDiffProps {
  fileDiff: FileDiffMetadata;
  /** Optional action slot rendered in a separate row above the diff content when expanded */
  actionSlot?: React.ReactNode;
}

/**
 * Renders a file diff with lazy loading.
 * The diff content is only rendered when the user expands the file section.
 * This improves performance for large diffs with many files.
 */
export function LazyFileDiff({ fileDiff, actionSlot }: LazyFileDiffProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const toggleExpanded = useCallback(() => {
    setIsExpanded((prev) => !prev);
  }, []);

  // Calculate additions and deletions from hunks (memoized to avoid recomputation on re-renders)
  const { additions, deletions } = useMemo(
    () =>
      (fileDiff.hunks ?? []).reduce(
        (acc, hunk) => {
          acc.additions += hunk.additionCount;
          acc.deletions += hunk.deletionCount;
          return acc;
        },
        { additions: 0, deletions: 0 }
      ),
    [fileDiff.hunks]
  );

  // Generate a stable ID for accessibility (aria-controls)
  const contentId = useMemo(
    () => `file-diff-${encodeURIComponent(fileDiff.name)}`,
    [fileDiff.name]
  );

  return (
    <div className="border rounded-lg overflow-hidden">
      {/* File header - always visible */}
      <button
        type="button"
        onClick={toggleExpanded}
        aria-expanded={isExpanded}
        aria-controls={contentId}
        aria-label={`${isExpanded ? "Collapse" : "Expand"} diff for ${fileDiff.name}`}
        className="w-full flex items-center gap-2 px-3 py-2 bg-muted/50 hover:bg-muted transition-colors text-left"
      >
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
        )}
        <File className="h-4 w-4 text-muted-foreground shrink-0" />
        <span className="font-mono text-sm truncate flex-1">{fileDiff.name}</span>
        <div className="flex items-center gap-2 text-sm shrink-0">
          {additions > 0 && (
            <span className="flex items-center gap-0.5 text-green-600 dark:text-green-400">
              <Plus className="h-3 w-3" />
              {additions}
            </span>
          )}
          {deletions > 0 && (
            <span className="flex items-center gap-0.5 text-red-600 dark:text-red-400">
              <Minus className="h-3 w-3" />
              {deletions}
            </span>
          )}
        </div>
      </button>

      {/* Diff content - only rendered when expanded */}
      {isExpanded && (
        <div id={contentId} role="region" aria-label={`Diff content for ${fileDiff.name}`}>
          {actionSlot && (
            <div className="flex justify-end px-3 py-2 border-b bg-background">
              {actionSlot}
            </div>
          )}
          <FileDiff
            fileDiff={fileDiff}
            options={{
              theme: { dark: "github-dark", light: "github-light" },
              diffStyle: "split",
            }}
          />
        </div>
      )}
    </div>
  );
}
