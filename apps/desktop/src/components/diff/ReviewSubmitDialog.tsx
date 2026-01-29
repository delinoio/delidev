import { useState } from "react";
import {
  CheckCircle,
  MessageCircle,
  XCircle,
  Loader2,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "../ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";
import { Label } from "../ui/label";
import { useReviewStore, ReviewAction, type InlineComment } from "../../stores";
import { cn } from "../../lib/utils";

interface ReviewSubmitDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  taskId: string;
  onSubmit: (action: ReviewAction, body: string, comments: InlineComment[]) => Promise<void>;
}

const reviewActionConfig = {
  [ReviewAction.Approve]: {
    icon: CheckCircle,
    label: "Approve",
    description: "Approve and proceed with the changes",
    color: "text-green-500",
    bgColor: "bg-green-500/10",
    borderColor: "border-green-500/50",
    buttonColor: "bg-green-600",
    indicatorColor: "bg-green-500",
  },
  [ReviewAction.RequestChanges]: {
    icon: XCircle,
    label: "Request Changes",
    description: "Request modifications before proceeding",
    color: "text-orange-500",
    bgColor: "bg-orange-500/10",
    borderColor: "border-orange-500/50",
    buttonColor: "bg-orange-600",
    indicatorColor: "bg-orange-500",
  },
  [ReviewAction.Comment]: {
    icon: MessageCircle,
    label: "Comment",
    description: "Leave feedback without approving or requesting changes",
    color: "text-blue-500",
    bgColor: "bg-blue-500/10",
    borderColor: "border-blue-500/50",
    buttonColor: "bg-blue-600",
    indicatorColor: "bg-blue-500",
  },
};

export function ReviewSubmitDialog({
  open,
  onOpenChange,
  taskId,
  onSubmit,
}: ReviewSubmitDialogProps) {
  const [isSubmitting, setIsSubmitting] = useState(false);

  const {
    getTaskReview,
    setReviewBody,
    setReviewAction,
    getAllComments,
    clearTaskReview,
  } = useReviewStore();

  const review = getTaskReview(taskId);
  const comments = getAllComments(taskId);

  const handleSubmit = async () => {
    setIsSubmitting(true);
    try {
      await onSubmit(review.reviewAction, review.reviewBody, comments);
      clearTaskReview(taskId);
      onOpenChange(false);
    } catch (error) {
      toast.error(
        error instanceof Error
          ? error.message
          : "Failed to submit review. Please try again."
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Submit Review</DialogTitle>
          <DialogDescription>
            Choose a review action and optionally add a comment.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Review Action Selection */}
          <div className="space-y-2">
            <Label>Review Action</Label>
            <div className="grid gap-2">
              {Object.values(ReviewAction).map((action) => {
                const config = reviewActionConfig[action];
                const Icon = config.icon;
                const isSelected = review.reviewAction === action;

                return (
                  <button
                    key={action}
                    type="button"
                    onClick={() => setReviewAction(taskId, action)}
                    className={cn(
                      "flex items-center gap-3 p-3 rounded-lg border-2 transition-all text-left",
                      isSelected
                        ? `${config.bgColor} ${config.borderColor}`
                        : "border-border hover:border-muted-foreground/50"
                    )}
                  >
                    <Icon className={cn("h-5 w-5", config.color)} />
                    <div className="flex-1">
                      <div className="font-medium">{config.label}</div>
                      <div className="text-xs text-muted-foreground">
                        {config.description}
                      </div>
                    </div>
                    {isSelected && (
                      <div className={cn("h-2 w-2 rounded-full", config.indicatorColor)} />
                    )}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Review Body */}
          <div className="space-y-2">
            <Label htmlFor="review-body">
              Comment (optional)
            </Label>
            <textarea
              id="review-body"
              value={review.reviewBody}
              onChange={(e) => setReviewBody(taskId, e.target.value)}
              placeholder="Leave a comment about the overall changes..."
              className="w-full min-h-[100px] p-3 text-sm bg-background border rounded-md resize-y"
            />
          </div>

          {/* Inline Comments Summary */}
          {comments.length > 0 && (
            <div className="rounded-lg border p-3 bg-muted/50">
              <div className="text-sm font-medium mb-2">
                Inline Comments ({comments.length})
              </div>
              <ul className="text-xs text-muted-foreground space-y-1 max-h-32 overflow-y-auto">
                {comments.map((comment) => (
                  <li key={comment.id} className="truncate">
                    <span className="font-mono">{comment.filePath}:{comment.line}</span>
                    {" - "}
                    {comment.body.substring(0, 50)}
                    {comment.body.length > 50 && "..."}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={isSubmitting}
          >
            Cancel
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={isSubmitting}
            className={cn(
              reviewActionConfig[review.reviewAction].buttonColor,
              "text-white hover:opacity-90"
            )}
          >
            {isSubmitting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <>
                {(() => {
                  const Icon = reviewActionConfig[review.reviewAction].icon;
                  return <Icon className="h-4 w-4" />;
                })()}
              </>
            )}
            {reviewActionConfig[review.reviewAction].label}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
