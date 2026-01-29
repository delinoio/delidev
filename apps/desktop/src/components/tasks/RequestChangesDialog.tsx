import { useState } from "react";
import { Loader2, RotateCcw } from "lucide-react";
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
import { Textarea } from "../ui/textarea";

interface RequestChangesDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (feedback: string) => Promise<void>;
}

export function RequestChangesDialog({
  open,
  onOpenChange,
  onSubmit,
}: RequestChangesDialogProps) {
  const [feedback, setFeedback] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async () => {
    if (!feedback.trim()) return;

    setIsSubmitting(true);
    try {
      await onSubmit(feedback);
      setFeedback("");
      onOpenChange(false);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleOpenChange = (newOpen: boolean) => {
    if (!isSubmitting) {
      if (!newOpen) {
        setFeedback("");
      }
      onOpenChange(newOpen);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Request Changes</DialogTitle>
          <DialogDescription>
            Describe the changes you want the AI agent to make. This feedback
            will be appended to the original task prompt and the agent will
            address your concerns.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="feedback">Feedback</Label>
            <Textarea
              id="feedback"
              value={feedback}
              onChange={(e) => setFeedback(e.target.value)}
              placeholder="Describe the changes you want..."
              className="min-h-[120px] resize-y"
              disabled={isSubmitting}
              autoFocus
            />
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={isSubmitting}
          >
            Cancel
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={!feedback.trim() || isSubmitting}
          >
            {isSubmitting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RotateCcw className="h-4 w-4" />
            )}
            Request Changes
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
