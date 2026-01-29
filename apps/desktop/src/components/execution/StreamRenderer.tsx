import { useState, useMemo, useEffect } from "react";
import { Card } from "../ui/card";
import { Badge } from "../ui/badge";
import {
  MessageSquare,
  Wrench,
  CheckCircle,
  AlertCircle,
  Terminal,
  Play,
  ChevronDown,
  ChevronRight,
  User,
  Cpu,
  Bot,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeRaw from "rehype-raw";
import rehypeSanitize from "rehype-sanitize";
import type {
  ClaudeStreamEvent,
  ClaudeStreamMessage,
  ClaudeStreamAssistant,
  ClaudeStreamUser,
  ClaudeStreamResult,
  ClaudeStreamSystem,
  ContentBlock,
} from "../../api";

// Generate a stable key from text content using a simple hash
function generateTextKey(text: string, index: number): string {
  // Use first 50 chars + length as a simple content-based key with index fallback for duplicates
  const contentKey = `${text.slice(0, 50)}-${text.length}`;
  return `text-${index}-${contentKey}`;
}

export interface StreamEntry {
  id: string;
  timestamp: string;
  message: ClaudeStreamMessage;
}

interface StreamRendererProps {
  entries: StreamEntry[];
  taskStatus?: string;
}

function getParentToolUseId(message: ClaudeStreamMessage): string | null {
  if ("parent_tool_use_id" in message && message.parent_tool_use_id) {
    return message.parent_tool_use_id;
  }
  return null;
}

interface GroupedEntry {
  type: "normal" | "subagent";
  entry?: StreamEntry;
  taskToolUseId?: string;
  taskDescription?: string;
  subagentEntries?: StreamEntry[];
}

function groupEntriesBySubagent(entries: StreamEntry[]): GroupedEntry[] {
  const result: GroupedEntry[] = [];
  const taskToolUseIds = new Map<string, { description: string; index: number }>();

  // First pass: find all Task tool uses and their positions
  entries.forEach((entry, idx) => {
    if (entry.message.type === "assistant") {
      const content = entry.message.message.content || [];
      for (const block of content) {
        if (block.type === "tool_use" && block.name === "Task") {
          const input = block.input as Record<string, unknown>;
          const description = (input.description as string) || "SubAgent";
          taskToolUseIds.set(block.id, { description, index: idx });
        }
      }
    }
  });

  // Second pass: group entries
  const subagentGroups = new Map<string, StreamEntry[]>();
  const processedIndices = new Set<number>();

  entries.forEach((entry, idx) => {
    const parentId = getParentToolUseId(entry.message);
    if (parentId && taskToolUseIds.has(parentId)) {
      // This entry belongs to a subagent
      if (!subagentGroups.has(parentId)) {
        subagentGroups.set(parentId, []);
      }
      const group = subagentGroups.get(parentId);
      if (group) {
        group.push(entry);
      }
      processedIndices.add(idx);
    }
  });

  // Third pass: build result with subagent groups inserted after Task tool use
  entries.forEach((entry, idx) => {
    if (processedIndices.has(idx)) {
      return; // Skip entries already grouped as subagent
    }

    // Check if this entry contains a Task tool use
    if (entry.message.type === "assistant") {
      const content = entry.message.message.content || [];
      const taskBlocks = content.filter(
        (b) => b.type === "tool_use" && b.name === "Task"
      );

      if (taskBlocks.length > 0) {
        // Split: non-Task blocks as normal entry, Task blocks as subagent groups
        const nonTaskBlocks = content.filter(
          (b) => !(b.type === "tool_use" && b.name === "Task")
        );

        if (nonTaskBlocks.length > 0) {
          result.push({
            type: "normal",
            entry: {
              ...entry,
              message: {
                ...entry.message,
                message: { ...entry.message.message, content: nonTaskBlocks },
              },
            },
          });
        }

        // Add subagent groups for each Task tool use
        for (const block of taskBlocks) {
          if (block.type === "tool_use") {
            const info = taskToolUseIds.get(block.id);
            const subEntries = subagentGroups.get(block.id) || [];
            result.push({
              type: "subagent",
              taskToolUseId: block.id,
              taskDescription: info?.description || "SubAgent",
              subagentEntries: subEntries,
            });
          }
        }
      } else {
        result.push({ type: "normal", entry });
      }
    } else {
      result.push({ type: "normal", entry });
    }
  });

  return result;
}

// Task statuses that indicate the task is finished
const FINISHED_TASK_STATUSES = ["done", "rejected", "pr_open", "in_review", "approved"] as const;

export function StreamRenderer({ entries, taskStatus }: StreamRendererProps) {
  const groupedEntries = useMemo(() => groupEntriesBySubagent(entries), [entries]);

  // Determine if logs should be expanded by default
  // Expand for in-progress tasks, collapse for finished tasks
  const isTaskFinished = taskStatus !== undefined && FINISHED_TASK_STATUSES.includes(taskStatus as (typeof FINISHED_TASK_STATUSES)[number]);
  const [isExpanded, setIsExpanded] = useState(!isTaskFinished);

  // Sync expanded state when task transitions to finished
  useEffect(() => {
    if (isTaskFinished) {
      setIsExpanded(false);
    }
  }, [isTaskFinished]);

  if (entries.length === 0) {
    return null;
  }

  // Find the result entry to show completion status in collapsed view
  const resultEntry = entries.find(e => e.message.type === "result");
  const resultMessage = resultEntry?.message as ClaudeStreamResult | undefined;
  const isSuccess = resultMessage && !resultMessage.is_error;
  const durationSec = resultMessage?.duration_ms ? (resultMessage.duration_ms / 1000).toFixed(1) : null;

  return (
    <div className="space-y-2">
      {/* Collapsible header for completed sessions */}
      {isTaskFinished && (
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="w-full flex items-center gap-3 p-3 rounded-lg border bg-muted/50 hover:bg-muted transition-colors text-left"
          aria-expanded={isExpanded}
          aria-label={isExpanded ? "Collapse session logs" : "Expand session logs"}
        >
          {isSuccess !== undefined ? (
            isSuccess ? (
              <CheckCircle className="h-5 w-5 text-green-600 dark:text-green-400 flex-shrink-0" />
            ) : (
              <AlertCircle className="h-5 w-5 text-red-600 dark:text-red-400 flex-shrink-0" />
            )
          ) : (
            <Terminal className="h-5 w-5 text-muted-foreground flex-shrink-0" />
          )}
          <div className="flex-1 flex items-center gap-2 min-w-0">
            <span className="font-medium">
              {isSuccess !== undefined
                ? isSuccess
                  ? "Session Completed Successfully"
                  : "Session Failed"
                : "Claude Code Session"}
            </span>
            <span className="text-xs text-muted-foreground">
              ({entries.length} messages)
            </span>
            {durationSec && (
              <span className="text-xs text-muted-foreground">
                {durationSec}s
              </span>
            )}
            {resultMessage?.cost_usd && (
              <span className="text-xs text-muted-foreground">
                ${resultMessage.cost_usd.toFixed(4)}
              </span>
            )}
          </div>
          {isExpanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground flex-shrink-0" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
          )}
        </button>
      )}

      {/* Session content - always shown for in-progress, collapsible for finished */}
      {(!isTaskFinished || isExpanded) && (
        <div className={isTaskFinished ? "pl-4 border-l-2 border-muted" : ""}>
          {groupedEntries.map((group, idx) => {
            if (group.type === "subagent") {
              return (
                <SubagentGroup
                  key={group.taskToolUseId || idx}
                  taskToolUseId={group.taskToolUseId!}
                  description={group.taskDescription!}
                  entries={group.subagentEntries || []}
                  defaultExpanded={false}
                />
              );
            } else {
              return <StreamEntryCard key={group.entry!.id} entry={group.entry!} />;
            }
          })}
        </div>
      )}
    </div>
  );
}

function SubagentGroup({
  taskToolUseId,
  description,
  entries,
  defaultExpanded = false,
}: {
  taskToolUseId: string;
  description: string;
  entries: StreamEntry[];
  defaultExpanded?: boolean;
}) {
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);

  return (
    <Card className="p-3 border-l-4 border-l-purple-500 bg-purple-50/50 dark:bg-purple-900/10">
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-3 text-left"
      >
        <Bot className="h-5 w-5 text-purple-500 flex-shrink-0" />
        <div className="flex-1 flex items-center gap-2 min-w-0">
          <Badge variant="secondary" className="bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300">
            SubAgent
          </Badge>
          <span className="text-sm text-muted-foreground truncate">{description}</span>
          <span className="text-xs text-muted-foreground">
            ({entries.length} messages)
          </span>
        </div>
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground flex-shrink-0" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground flex-shrink-0" />
        )}
      </button>
      {isExpanded && entries.length > 0 && (
        <div className="mt-3 pl-4 border-l-2 border-purple-200 dark:border-purple-800 space-y-2">
          {entries.map((entry) => (
            <StreamEntryCard key={entry.id} entry={entry} />
          ))}
        </div>
      )}
    </Card>
  );
}

function StreamEntryCard({ entry }: { entry: StreamEntry }) {
  switch (entry.message.type) {
    case "system":
      return <SystemCard entry={entry} message={entry.message} />;
    case "assistant":
      return <AssistantCard entry={entry} message={entry.message} />;
    case "user":
      return <UserCard entry={entry} message={entry.message} />;
    case "result":
      return <ResultCard entry={entry} message={entry.message} />;
    default:
      return null;
  }
}

function SystemCard({
  entry,
  message,
}: {
  entry: StreamEntry;
  message: ClaudeStreamSystem;
}) {
  return (
    <div className="flex items-center gap-2 text-sm text-muted-foreground py-2 px-1">
      <Play className="h-4 w-4" />
      <span>
        {message.subtype === "init" ? "Session initialized" : message.subtype}
      </span>
      <span className="text-xs">
        {new Date(entry.timestamp).toLocaleTimeString()}
      </span>
    </div>
  );
}

function AssistantCard({
  entry,
  message,
}: {
  entry: StreamEntry;
  message: ClaudeStreamAssistant;
}) {
  const content = message.message.content || [];

  // Separate text blocks and tool blocks
  const textBlocks = content.filter((b) => b.type === "text");
  const toolBlocks = content.filter((b) => b.type === "tool_use");

  return (
    <div className="space-y-2">
      {/* Text content */}
      {textBlocks.length > 0 && (
        <Card className="p-4">
          <div className="flex items-start gap-3">
            <Cpu className="h-5 w-5 text-primary mt-0.5 flex-shrink-0" />
            <div className="flex-1 space-y-2 min-w-0">
              <div className="flex items-center gap-2 flex-wrap">
                <Badge variant="secondary">Claude</Badge>
                <span className="text-xs text-muted-foreground">
                  {new Date(entry.timestamp).toLocaleTimeString()}
                </span>
              </div>
              <div className="text-sm prose prose-sm dark:prose-invert max-w-none break-words">
                {textBlocks.map((block, idx) =>
                  block.type === "text" ? (
                    <ReactMarkdown
                      key={generateTextKey(block.text, idx)}
                      remarkPlugins={[remarkGfm]}
                      rehypePlugins={[rehypeRaw, rehypeSanitize]}
                    >
                      {block.text}
                    </ReactMarkdown>
                  ) : null
                )}
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* Tool use blocks */}
      {toolBlocks.map((block, idx) =>
        block.type === "tool_use" ? (
          <ToolUseCard key={block.id || idx} block={block} timestamp={entry.timestamp} />
        ) : null
      )}
    </div>
  );
}

function ToolUseCard({
  block,
  timestamp,
}: {
  block: { type: "tool_use"; id: string; name: string; input: Record<string, unknown> };
  timestamp: string;
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Format input for display
  const inputStr = JSON.stringify(block.input, null, 2);
  const isLongInput = inputStr.length > 200;

  return (
    <Card className="p-4 border-l-4 border-l-blue-500">
      <div className="flex items-start gap-3">
        <Wrench className="h-5 w-5 text-blue-500 mt-0.5 flex-shrink-0" />
        <div className="flex-1 space-y-2 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <Badge variant="info">{block.name}</Badge>
            <span className="text-xs text-muted-foreground">
              {new Date(timestamp).toLocaleTimeString()}
            </span>
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-1"
            >
              {isExpanded ? (
                <ChevronDown className="h-3 w-3" />
              ) : (
                <ChevronRight className="h-3 w-3" />
              )}
              {isExpanded ? "Hide" : "Show"} input
            </button>
          </div>
          {isExpanded && (
            <pre className="text-xs bg-muted p-2 rounded overflow-x-auto max-h-64 overflow-y-auto">
              {inputStr}
            </pre>
          )}
        </div>
      </div>
    </Card>
  );
}

function UserCard({
  entry,
  message,
}: {
  entry: StreamEntry;
  message: ClaudeStreamUser;
}) {
  const content = message.message.content || [];

  // Check for tool results
  const toolResults = content.filter((b) => b.type === "tool_result");
  const textBlocks = content.filter((b) => b.type === "text");

  return (
    <div className="space-y-2">
      {/* Tool results */}
      {toolResults.map((block, idx) =>
        block.type === "tool_result" ? (
          <ToolResultCard
            key={block.tool_use_id || idx}
            block={block}
            timestamp={entry.timestamp}
          />
        ) : null
      )}

      {/* Text content (if any) */}
      {textBlocks.length > 0 && (
        <Card className="p-4 bg-muted/50">
          <div className="flex items-start gap-3">
            <User className="h-5 w-5 text-muted-foreground mt-0.5 flex-shrink-0" />
            <div className="flex-1 space-y-2 min-w-0">
              <div className="flex items-center gap-2">
                <Badge variant="outline">User</Badge>
                <span className="text-xs text-muted-foreground">
                  {new Date(entry.timestamp).toLocaleTimeString()}
                </span>
              </div>
              <div className="text-sm prose prose-sm dark:prose-invert max-w-none break-words">
                {textBlocks.map((block, idx) =>
                  block.type === "text" ? (
                    <ReactMarkdown
                      key={generateTextKey(block.text, idx)}
                      remarkPlugins={[remarkGfm]}
                      rehypePlugins={[rehypeRaw, rehypeSanitize]}
                    >
                      {block.text}
                    </ReactMarkdown>
                  ) : null
                )}
              </div>
            </div>
          </div>
        </Card>
      )}
    </div>
  );
}

function ToolResultCard({
  block,
  timestamp,
}: {
  block: { type: "tool_result"; tool_use_id: string; content?: string; is_error?: boolean };
  timestamp: string;
}) {
  const [isExpanded, setIsExpanded] = useState(false);

  const output = typeof block.content === "string" ? block.content : block.content ? JSON.stringify(block.content) : "";
  const lines = output.split("\n");
  const isLong = lines.length > 10;
  const displayOutput = isExpanded || !isLong ? output : lines.slice(0, 10).join("\n") + "\n...";
  const isError = block.is_error;

  return (
    <Card
      className={`p-4 bg-zinc-950 text-zinc-300 border-l-4 ${
        isError ? "border-l-red-500" : "border-l-green-500"
      }`}
    >
      <div className="flex items-start gap-3">
        <Terminal
          className={`h-5 w-5 mt-0.5 flex-shrink-0 ${
            isError ? "text-red-500" : "text-green-500"
          }`}
        />
        <div className="flex-1 space-y-2 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <Badge variant={isError ? "destructive" : "success"}>
              {isError ? "Error" : "Result"}
            </Badge>
            {isLong && (
              <button
                onClick={() => setIsExpanded(!isExpanded)}
                className="text-xs text-zinc-500 hover:text-zinc-300 flex items-center gap-1"
              >
                {isExpanded ? (
                  <ChevronDown className="h-3 w-3" />
                ) : (
                  <ChevronRight className="h-3 w-3" />
                )}
                {isExpanded ? "Collapse" : "Expand"}
              </button>
            )}
          </div>
          {output && (
            <pre className="text-xs font-mono whitespace-pre-wrap break-all overflow-x-auto max-h-96 overflow-y-auto">
              {displayOutput}
            </pre>
          )}
        </div>
      </div>
    </Card>
  );
}

function ResultCard({
  entry,
  message,
}: {
  entry: StreamEntry;
  message: ClaudeStreamResult;
}) {
  const isSuccess = !message.is_error;
  const durationSec = message.duration_ms ? (message.duration_ms / 1000).toFixed(1) : null;

  return (
    <Card
      className={`p-4 ${
        isSuccess
          ? "bg-green-50 border-green-200 dark:bg-green-900/20 dark:border-green-800"
          : "bg-red-50 border-red-200 dark:bg-red-900/20 dark:border-red-800"
      }`}
    >
      <div className="flex items-center gap-3">
        {isSuccess ? (
          <CheckCircle className="h-5 w-5 text-green-600 dark:text-green-400" />
        ) : (
          <AlertCircle className="h-5 w-5 text-red-600 dark:text-red-400" />
        )}
        <div className="flex-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span
              className={`font-medium ${
                isSuccess
                  ? "text-green-800 dark:text-green-200"
                  : "text-red-800 dark:text-red-200"
              }`}
            >
              {isSuccess ? "Completed Successfully" : "Execution Failed"}
            </span>
            {durationSec && (
              <span className="text-xs text-muted-foreground">({durationSec}s)</span>
            )}
            {message.cost_usd && (
              <span className="text-xs text-muted-foreground">
                (${message.cost_usd.toFixed(4)})
              </span>
            )}
          </div>
          {message.result && (
            <div className="text-sm mt-1 text-muted-foreground prose prose-sm dark:prose-invert max-w-none">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                rehypePlugins={[rehypeRaw, rehypeSanitize]}
              >
                {message.result}
              </ReactMarkdown>
            </div>
          )}
        </div>
      </div>
    </Card>
  );
}
