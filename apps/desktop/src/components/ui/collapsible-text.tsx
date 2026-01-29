import * as React from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { cn } from "../../lib/utils";

interface CollapsibleTextProps {
  text: string;
  /** Maximum number of characters to show before collapsing. Default: 300 */
  maxLength?: number;
  /** Maximum number of lines to show before collapsing. Default: 5 */
  maxLines?: number;
  className?: string;
}

/**
 * A component that displays text with a "Show more/less" toggle
 * when the text exceeds a certain length or number of lines.
 */
export function CollapsibleText({
  text,
  maxLength = 300,
  maxLines = 5,
  className,
}: CollapsibleTextProps) {
  const [isExpanded, setIsExpanded] = React.useState(false);

  // Check if text exceeds the limits
  const lineCount = text.split("\n").length;
  const shouldCollapse = text.length > maxLength || lineCount > maxLines;

  if (!shouldCollapse) {
    return <p className={cn("whitespace-pre-wrap", className)}>{text}</p>;
  }

  // Truncate text if collapsed
  const truncatedText = React.useMemo(() => {
    if (isExpanded) return text;

    const lines = text.split("\n");
    let result = "";
    let charCount = 0;

    for (let i = 0; i < lines.length && i < maxLines; i++) {
      const line = lines[i];
      const remainingChars = maxLength - charCount;

      if (remainingChars <= 0) break;

      if (line.length > remainingChars) {
        result += (i > 0 ? "\n" : "") + line.slice(0, remainingChars);
        break;
      }

      result += (i > 0 ? "\n" : "") + line;
      charCount += line.length + 1; // +1 for newline
    }

    return result + "...";
  }, [text, maxLength, maxLines, isExpanded]);

  return (
    <div className={className}>
      <p className="whitespace-pre-wrap">{truncatedText}</p>
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="mt-1 flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors"
        aria-expanded={isExpanded}
      >
        {isExpanded ? (
          <>
            <ChevronDown className="h-4 w-4" />
            Show less
          </>
        ) : (
          <>
            <ChevronRight className="h-4 w-4" />
            Show more
          </>
        )}
      </button>
    </div>
  );
}
