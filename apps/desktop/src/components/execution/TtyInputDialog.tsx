import * as React from "react";
import { useState } from "react";
import { AlertCircle } from "lucide-react";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "../ui/card";
import { Button } from "../ui/button";
import { Textarea } from "../ui/textarea";
import { Label } from "../ui/label";
import type { TtyInputRequest } from "../../api";

interface TtyInputDialogProps {
  request: TtyInputRequest;
  onSubmit: (response: string) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

export function TtyInputDialog({
  request,
  onSubmit,
  onCancel,
  isSubmitting = false,
}: TtyInputDialogProps) {
  const [selectedOption, setSelectedOption] = useState<string>("");
  const [customResponse, setCustomResponse] = useState<string>("");
  const [useCustom, setUseCustom] = useState<boolean>(request.options.length === 0);

  const handleSubmit = () => {
    const response = useCustom ? customResponse : selectedOption;
    if (response.trim()) {
      onSubmit(response.trim());
    }
  };

  const handleOptionSelect = (label: string) => {
    setSelectedOption(label);
    setUseCustom(false);
  };

  const handleCustomFocus = () => {
    setUseCustom(true);
    setSelectedOption("");
  };

  const isValid = useCustom ? customResponse.trim() !== "" : selectedOption !== "";

  return (
    <Card className="border-2 border-amber-500/50 bg-amber-50/10">
      <CardHeader className="pb-3">
        <CardTitle className="flex items-center gap-2 text-lg">
          <AlertCircle className="h-5 w-5 text-amber-500" />
          Agent Question
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <p className="text-sm text-muted-foreground">
          The AI agent is asking:
        </p>
        <p className="text-sm font-medium whitespace-pre-wrap">
          "{request.prompt}"
        </p>

        {request.options.length > 0 && (
          <div className="space-y-2">
            <Label className="text-xs text-muted-foreground">Options:</Label>
            <div className="space-y-2">
              {request.options.map((option, index) => (
                <button
                  key={index}
                  type="button"
                  onClick={() => handleOptionSelect(option.label)}
                  className={`w-full text-left p-3 rounded-lg border transition-colors ${
                    selectedOption === option.label && !useCustom
                      ? "border-primary bg-primary/5"
                      : "border-border hover:border-primary/50 hover:bg-muted/50"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <div
                      className={`w-4 h-4 rounded-full border-2 flex items-center justify-center ${
                        selectedOption === option.label && !useCustom
                          ? "border-primary"
                          : "border-muted-foreground"
                      }`}
                    >
                      {selectedOption === option.label && !useCustom && (
                        <div className="w-2 h-2 rounded-full bg-primary" />
                      )}
                    </div>
                    <div className="flex-1">
                      <div className="font-medium text-sm">{option.label}</div>
                      {option.description && (
                        <div className="text-xs text-muted-foreground mt-0.5">
                          {option.description}
                        </div>
                      )}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="space-y-2">
          <Label htmlFor="custom-response" className="text-xs text-muted-foreground">
            {request.options.length > 0
              ? "Or provide a custom response:"
              : "Your response:"}
          </Label>
          <Textarea
            id="custom-response"
            value={customResponse}
            onChange={(e) => setCustomResponse(e.target.value)}
            onFocus={handleCustomFocus}
            placeholder="Type your response here..."
            className={`min-h-[80px] ${
              useCustom ? "ring-1 ring-primary" : ""
            }`}
            disabled={isSubmitting}
          />
        </div>
      </CardContent>
      <CardFooter className="flex justify-end gap-2 pt-0">
        <Button
          variant="outline"
          onClick={onCancel}
          disabled={isSubmitting}
        >
          Cancel
        </Button>
        <Button
          onClick={handleSubmit}
          disabled={!isValid || isSubmitting}
        >
          {isSubmitting ? "Submitting..." : "Submit Response"}
        </Button>
      </CardFooter>
    </Card>
  );
}
