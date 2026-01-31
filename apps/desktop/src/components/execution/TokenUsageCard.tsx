import { useEffect, useState } from "react";
import { Coins, Clock, MessageSquare, AlertCircle, Loader2 } from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import type { TokenUsageSummary } from "../../api";
import * as api from "../../api";

interface TokenUsageCardProps {
  taskId: string;
  taskType: "unit" | "composite";
}

export function TokenUsageCard({ taskId, taskType }: TokenUsageCardProps) {
  const [usage, setUsage] = useState<TokenUsageSummary | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchUsage = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const summary = taskType === "unit"
          ? await api.getUnitTaskTokenUsageSummary(taskId)
          : await api.getCompositeTaskTokenUsageSummary(taskId);
        setUsage(summary);
      } catch (err) {
        console.error("Failed to fetch token usage:", err);
        setError(err instanceof Error ? err.message : "Failed to fetch token usage");
      } finally {
        setIsLoading(false);
      }
    };

    fetchUsage();
  }, [taskId, taskType]);

  // Don't show the card if there's no usage data and no sessions
  if (!isLoading && usage && usage.session_count === 0) {
    return null;
  }

  const formatCost = (cost: number) => {
    if (cost < 0.01) {
      return `$${cost.toFixed(4)}`;
    }
    return `$${cost.toFixed(2)}`;
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) {
      return `${Math.round(ms)}ms`;
    }
    const seconds = ms / 1000;
    if (seconds < 60) {
      return `${seconds.toFixed(1)}s`;
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = Math.round(seconds % 60);
    return `${minutes}m ${remainingSeconds}s`;
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Coins className="h-5 w-5" />
          Token Usage
        </CardTitle>
        <CardDescription>
          AI agent execution cost and performance
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : error ? (
          <div className="text-sm text-destructive">{error}</div>
        ) : usage ? (
          <div className="grid grid-cols-2 gap-4">
            {/* Cost */}
            <div className="rounded-lg border p-3">
              <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
                <Coins className="h-4 w-4" />
                Total Cost
              </div>
              <div className="text-2xl font-semibold">
                {formatCost(usage.total_cost_usd)}
              </div>
            </div>

            {/* Duration */}
            <div className="rounded-lg border p-3">
              <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
                <Clock className="h-4 w-4" />
                Total Duration
              </div>
              <div className="text-2xl font-semibold">
                {formatDuration(usage.total_duration_ms)}
              </div>
            </div>

            {/* Turns */}
            <div className="rounded-lg border p-3">
              <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
                <MessageSquare className="h-4 w-4" />
                Total Turns
              </div>
              <div className="text-2xl font-semibold">
                {usage.total_num_turns}
              </div>
            </div>

            {/* Sessions */}
            <div className="rounded-lg border p-3">
              <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
                {usage.error_count > 0 ? (
                  <AlertCircle className="h-4 w-4 text-destructive" />
                ) : (
                  <MessageSquare className="h-4 w-4" />
                )}
                Sessions
              </div>
              <div className="text-2xl font-semibold">
                {usage.session_count}
                {usage.error_count > 0 && (
                  <span className="text-sm text-destructive ml-2">
                    ({usage.error_count} failed)
                  </span>
                )}
              </div>
            </div>
          </div>
        ) : (
          <div className="text-sm text-muted-foreground">
            No usage data available
          </div>
        )}
      </CardContent>
    </Card>
  );
}
