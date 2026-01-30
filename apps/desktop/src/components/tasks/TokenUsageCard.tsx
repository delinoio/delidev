import { useEffect, useState } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import { Loader2, Coins, ArrowDownToLine, ArrowUpFromLine, Hash, Layers } from "lucide-react";
import type { TaskUsageSummary } from "../../api";
import * as api from "../../api";

interface TokenUsageCardProps {
  taskId: string;
  taskType: "unit" | "composite";
}

/**
 * Formats a number with thousands separators
 */
function formatNumber(n: number): string {
  return n.toLocaleString();
}

/**
 * Formats USD currency value
 */
function formatCurrency(value: number): string {
  if (value < 0.01) {
    return value > 0 ? "<$0.01" : "$0.00";
  }
  return `$${value.toFixed(2)}`;
}

export function TokenUsageCard({ taskId, taskType }: TokenUsageCardProps) {
  const [usage, setUsage] = useState<TaskUsageSummary | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadUsage = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const result = taskType === "unit"
          ? await api.getUnitTaskUsage(taskId)
          : await api.getCompositeTaskUsage(taskId);
        setUsage(result);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load usage data");
      } finally {
        setIsLoading(false);
      }
    };

    loadUsage();
  }, [taskId, taskType]);

  // Don't show the card if there's no usage data
  if (!isLoading && !error && usage && usage.session_count === 0) {
    return null;
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Coins className="h-5 w-5" />
          Token Usage
        </CardTitle>
        <CardDescription>
          AI token consumption and estimated costs for this task.
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : error ? (
          <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
            <p className="text-sm text-destructive">{error}</p>
          </div>
        ) : usage ? (
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
            {/* Input Tokens */}
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <div className="rounded-full bg-blue-100 p-2 dark:bg-blue-900">
                <ArrowDownToLine className="h-4 w-4 text-blue-600 dark:text-blue-300" />
              </div>
              <div>
                <p className="text-sm font-medium text-muted-foreground">Input</p>
                <p className="text-lg font-semibold">{formatNumber(usage.total_input_tokens)}</p>
              </div>
            </div>

            {/* Output Tokens */}
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <div className="rounded-full bg-green-100 p-2 dark:bg-green-900">
                <ArrowUpFromLine className="h-4 w-4 text-green-600 dark:text-green-300" />
              </div>
              <div>
                <p className="text-sm font-medium text-muted-foreground">Output</p>
                <p className="text-lg font-semibold">{formatNumber(usage.total_output_tokens)}</p>
              </div>
            </div>

            {/* Total Tokens */}
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <div className="rounded-full bg-purple-100 p-2 dark:bg-purple-900">
                <Hash className="h-4 w-4 text-purple-600 dark:text-purple-300" />
              </div>
              <div>
                <p className="text-sm font-medium text-muted-foreground">Total</p>
                <p className="text-lg font-semibold">{formatNumber(usage.total_tokens)}</p>
              </div>
            </div>

            {/* Sessions */}
            <div className="flex items-center gap-3 rounded-lg border p-3">
              <div className="rounded-full bg-orange-100 p-2 dark:bg-orange-900">
                <Layers className="h-4 w-4 text-orange-600 dark:text-orange-300" />
              </div>
              <div>
                <p className="text-sm font-medium text-muted-foreground">Sessions</p>
                <p className="text-lg font-semibold">{usage.session_count}</p>
              </div>
            </div>

            {/* Cost */}
            <div className="flex items-center gap-3 rounded-lg border p-3 bg-primary/5">
              <div className="rounded-full bg-primary/20 p-2">
                <Coins className="h-4 w-4 text-primary" />
              </div>
              <div>
                <p className="text-sm font-medium text-muted-foreground">Est. Cost</p>
                <p className="text-lg font-semibold">{formatCurrency(usage.total_cost_usd)}</p>
              </div>
            </div>
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}
