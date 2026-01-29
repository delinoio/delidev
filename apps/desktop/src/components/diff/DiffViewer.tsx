import { useMemo, useState, useCallback } from "react";
import { FileDiff, type FileDiffMetadata } from "@pierre/diffs/react";
import { parsePatchFiles } from "@pierre/diffs";
import {
  ExternalLink,
  Loader2,
  Check,
  Eye,
  EyeOff,
  MessageSquarePlus,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "../ui/button";
import { Checkbox } from "../ui/checkbox";
import * as api from "../../api";
import { useReviewStore, type InlineComment } from "../../stores";
import { cn } from "../../lib/utils";
import { LazyFileDiff } from "./LazyFileDiff";

interface DiffViewerProps {
  diff: string;
  /** Repository path for opening files in editor */
  repoPath?: string;
  /** Base commit for diff comparison */
  baseCommit?: string;
  /** Head commit for diff comparison */
  headCommit?: string;
  /** Task ID for review state management */
  taskId?: string;
  /** Whether to show the file list sidebar */
  showFileList?: boolean;
  /** Whether to enable review features (viewed state, inline comments) */
  enableReviewFeatures?: boolean;
}

interface FileListItemProps {
  fileDiff: FileDiffMetadata;
  isSelected: boolean;
  isViewed: boolean;
  commentCount: number;
  onSelect: () => void;
  onToggleViewed: () => void;
}

function FileListItem({
  fileDiff,
  isSelected,
  isViewed,
  commentCount,
  onSelect,
  onToggleViewed,
}: FileListItemProps) {
  return (
    <div
      className={cn(
        "flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-accent/50 rounded-md transition-colors",
        isSelected && "bg-accent"
      )}
      onClick={onSelect}
    >
      <Checkbox
        checked={isViewed}
        onCheckedChange={() => {
          onToggleViewed();
        }}
        onClick={(e) => e.stopPropagation()}
        aria-label={isViewed ? "Mark as not viewed" : "Mark as viewed"}
      />
      <span
        className={cn(
          "flex-1 text-sm truncate",
          isViewed && "text-muted-foreground line-through"
        )}
        title={fileDiff.name}
      >
        {fileDiff.name}
      </span>
      {commentCount > 0 && (
        <span className="text-xs bg-primary/10 text-primary px-1.5 py-0.5 rounded">
          {commentCount}
        </span>
      )}
      {isViewed && <Check className="h-4 w-4 text-green-500 shrink-0" />}
    </div>
  );
}

interface InlineCommentFormProps {
  onSubmit: (body: string) => void;
  onCancel: () => void;
  initialValue?: string;
}

function InlineCommentForm({ onSubmit, onCancel, initialValue = "" }: InlineCommentFormProps) {
  const [body, setBody] = useState(initialValue);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (body.trim()) {
      onSubmit(body.trim());
    }
  };

  return (
    <form onSubmit={handleSubmit} className="p-3 bg-muted/50 rounded-md space-y-2">
      <textarea
        value={body}
        onChange={(e) => setBody(e.target.value)}
        placeholder="Add a comment..."
        className="w-full min-h-[80px] p-2 text-sm bg-background border rounded-md resize-y"
        aria-label="Comment text"
        autoFocus
      />
      <div className="flex gap-2 justify-end">
        <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" size="sm" disabled={!body.trim()}>
          Add comment
        </Button>
      </div>
    </form>
  );
}

interface InlineCommentDisplayProps {
  comment: InlineComment;
  onEdit: (body: string) => void;
  onDelete: () => void;
}

function InlineCommentDisplay({ comment, onEdit, onDelete }: InlineCommentDisplayProps) {
  const [isEditing, setIsEditing] = useState(false);

  if (isEditing) {
    return (
      <InlineCommentForm
        initialValue={comment.body}
        onSubmit={(body) => {
          onEdit(body);
          setIsEditing(false);
        }}
        onCancel={() => setIsEditing(false)}
      />
    );
  }

  return (
    <div className="p-3 bg-muted/50 rounded-md space-y-2">
      <p className="text-sm whitespace-pre-wrap">{comment.body}</p>
      <div className="flex gap-2 justify-end">
        <Button variant="ghost" size="sm" onClick={() => setIsEditing(true)}>
          Edit
        </Button>
        <Button variant="ghost" size="sm" onClick={onDelete} className="text-destructive">
          Delete
        </Button>
      </div>
    </div>
  );
}

export function DiffViewer({
  diff,
  repoPath,
  baseCommit,
  headCommit,
  taskId,
  showFileList = true,
  enableReviewFeatures = true,
}: DiffViewerProps) {
  const [openingFile, setOpeningFile] = useState<string | null>(null);
  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [isFileListCollapsed, setIsFileListCollapsed] = useState(false);
  const [commentingLine, setCommentingLine] = useState<{
    filePath: string;
    line: number;
    side: "left" | "right";
  } | null>(null);

  const {
    isFileViewed,
    toggleFileViewed,
    addInlineComment,
    removeInlineComment,
    updateInlineComment,
    getFileComments,
    getViewedFilesCount,
  } = useReviewStore();

  const files = useMemo(() => {
    const patches = parsePatchFiles(diff);
    return patches.flatMap((patch) => patch.files);
  }, [diff]);

  const handleOpenInEditor = async (filePath: string) => {
    setOpeningFile(filePath);
    try {
      await api.openInEditor({
        filePath,
        repoPath,
        baseCommit,
        headCommit,
      });
    } catch (error) {
      console.error("Failed to open file in editor:", error);
      toast.error(
        error instanceof Error
          ? error.message
          : "Failed to open file in editor. Is your editor installed and available in PATH?"
      );
    } finally {
      setOpeningFile(null);
    }
  };

  const handleToggleViewed = useCallback(
    (filePath: string) => {
      if (taskId) {
        toggleFileViewed(taskId, filePath);
      }
    },
    [taskId, toggleFileViewed]
  );

  const handleAddComment = useCallback(
    (filePath: string, line: number, side: "left" | "right", body: string) => {
      if (taskId) {
        addInlineComment(taskId, filePath, line, side, body);
        setCommentingLine(null);
      }
    },
    [taskId, addInlineComment]
  );

  const handleRemoveComment = useCallback(
    (commentId: string) => {
      if (taskId) {
        removeInlineComment(taskId, commentId);
      }
    },
    [taskId, removeInlineComment]
  );

  const handleUpdateComment = useCallback(
    (commentId: string, body: string) => {
      if (taskId) {
        updateInlineComment(taskId, commentId, body);
      }
    },
    [taskId, updateInlineComment]
  );

  const getFileCommentCount = useCallback(
    (filePath: string) => {
      if (!taskId) return 0;
      return getFileComments(taskId, filePath).length;
    },
    [taskId, getFileComments]
  );

  if (files.length === 0) {
    return (
      <p className="text-sm text-muted-foreground py-4">No changes to display.</p>
    );
  }

  const canOpenInEditor = repoPath && baseCommit && headCommit;
  const viewedCount = taskId ? getViewedFilesCount(taskId) : 0;
  const selectedFile = files[selectedFileIndex];
  const fileComments = taskId && selectedFile ? getFileComments(taskId, selectedFile.name) : [];

  // When showFileList is disabled, render all files using LazyFileDiff (collapsed by default)
  if (!showFileList) {
    return (
      <div className="space-y-2">
        {files.map((fileDiff, index) => (
          <LazyFileDiff
            key={`${fileDiff.name}-${index}`}
            fileDiff={fileDiff}
            actionSlot={
              canOpenInEditor && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => handleOpenInEditor(fileDiff.name)}
                  disabled={openingFile === fileDiff.name}
                  aria-busy={openingFile === fileDiff.name}
                >
                  <span className="inline-flex items-center gap-2">
                    {openingFile === fileDiff.name ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <ExternalLink className="h-4 w-4" />
                    )}
                    <span>Open in Editor</span>
                  </span>
                </Button>
              )
            }
          />
        ))}
      </div>
    );
  }

  return (
    <div className="flex gap-4">
      {/* File List Sidebar */}
      {showFileList && (
        <div
          className={cn(
            "shrink-0 border-r pr-4 transition-all",
            isFileListCollapsed ? "w-10" : "w-64"
          )}
        >
          <div className="flex items-center gap-2 mb-2">
            <Button
              variant="ghost"
              size="sm"
              className="p-1 h-auto"
              onClick={() => setIsFileListCollapsed(!isFileListCollapsed)}
            >
              {isFileListCollapsed ? (
                <ChevronRight className="h-4 w-4" />
              ) : (
                <ChevronDown className="h-4 w-4" />
              )}
            </Button>
            {!isFileListCollapsed && (
              <>
                <span className="text-sm font-medium flex-1">
                  Files ({files.length})
                </span>
                {enableReviewFeatures && taskId && (
                  <span className="text-xs text-muted-foreground">
                    {viewedCount}/{files.length} viewed
                  </span>
                )}
              </>
            )}
          </div>
          {!isFileListCollapsed && (
            <div className="space-y-1 max-h-[500px] overflow-y-auto">
              {files.map((fileDiff, index) => (
                <FileListItem
                  key={`${fileDiff.name}-${index}`}
                  fileDiff={fileDiff}
                  isSelected={selectedFileIndex === index}
                  isViewed={
                    enableReviewFeatures && taskId
                      ? isFileViewed(taskId, fileDiff.name)
                      : false
                  }
                  commentCount={
                    enableReviewFeatures ? getFileCommentCount(fileDiff.name) : 0
                  }
                  onSelect={() => setSelectedFileIndex(index)}
                  onToggleViewed={() => handleToggleViewed(fileDiff.name)}
                />
              ))}
            </div>
          )}
        </div>
      )}

      {/* Main Diff View */}
      <div className="flex-1 min-w-0">
        {selectedFile && (
          <div key={`${selectedFile.name}-${selectedFileIndex}`}>
            {/* File Header */}
            <div className="flex items-center justify-between mb-2 pb-2 border-b">
              <div className="flex items-center gap-2">
                <span className="font-medium text-sm">{selectedFile.name}</span>
                {enableReviewFeatures && taskId && isFileViewed(taskId, selectedFile.name) && (
                  <span className="text-xs bg-green-500/10 text-green-500 px-2 py-0.5 rounded flex items-center gap-1">
                    <Eye className="h-3 w-3" />
                    Viewed
                  </span>
                )}
              </div>
              <div className="flex items-center gap-2">
                {enableReviewFeatures && taskId && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleToggleViewed(selectedFile.name)}
                  >
                    {isFileViewed(taskId, selectedFile.name) ? (
                      <>
                        <EyeOff className="h-4 w-4" />
                        Mark as not viewed
                      </>
                    ) : (
                      <>
                        <Eye className="h-4 w-4" />
                        Mark as viewed
                      </>
                    )}
                  </Button>
                )}
                {canOpenInEditor && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleOpenInEditor(selectedFile.name)}
                    disabled={openingFile === selectedFile.name}
                    aria-busy={openingFile === selectedFile.name}
                  >
                    <span className="inline-flex items-center gap-2">
                      {openingFile === selectedFile.name ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <ExternalLink className="h-4 w-4" />
                      )}
                      <span>Open in Editor</span>
                    </span>
                  </Button>
                )}
              </div>
            </div>

            {/* Diff Content */}
            <FileDiff
              fileDiff={selectedFile}
              options={{
                theme: { dark: "github-dark", light: "github-light" },
                diffStyle: "split",
              }}
            />

            {/* Inline Comments Section */}
            {enableReviewFeatures && taskId && (
              <div className="mt-4 space-y-3">
                {fileComments.length > 0 && (
                  <div className="space-y-2">
                    <h4 className="text-sm font-medium flex items-center gap-2">
                      <MessageSquarePlus className="h-4 w-4" />
                      Comments on this file ({fileComments.length})
                    </h4>
                    {fileComments.map((comment) => (
                      <div key={comment.id} className="pl-4 border-l-2 border-primary/30">
                        <div className="text-xs text-muted-foreground mb-1">
                          Line {comment.line} ({comment.side === "left" ? "old" : "new"})
                        </div>
                        <InlineCommentDisplay
                          comment={comment}
                          onEdit={(body) => handleUpdateComment(comment.id, body)}
                          onDelete={() => handleRemoveComment(comment.id)}
                        />
                      </div>
                    ))}
                  </div>
                )}

                {/* Add Comment Form */}
                {commentingLine?.filePath === selectedFile.name ? (
                  <InlineCommentForm
                    onSubmit={(body) =>
                      handleAddComment(
                        commentingLine.filePath,
                        commentingLine.line,
                        commentingLine.side,
                        body
                      )
                    }
                    onCancel={() => setCommentingLine(null)}
                  />
                ) : (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      setCommentingLine({
                        filePath: selectedFile.name,
                        line: 1,
                        side: "right",
                      })
                    }
                  >
                    <MessageSquarePlus className="h-4 w-4" />
                    Add file comment
                  </Button>
                )}
              </div>
            )}
          </div>
        )}

        {/* Navigation between files */}
        {files.length > 1 && (
          <div className="flex items-center justify-between mt-4 pt-4 border-t">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setSelectedFileIndex(Math.max(0, selectedFileIndex - 1))}
              disabled={selectedFileIndex === 0}
            >
              Previous file
            </Button>
            <span className="text-sm text-muted-foreground">
              {selectedFileIndex + 1} of {files.length}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() =>
                setSelectedFileIndex(Math.min(files.length - 1, selectedFileIndex + 1))
              }
              disabled={selectedFileIndex === files.length - 1}
            >
              Next file
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
