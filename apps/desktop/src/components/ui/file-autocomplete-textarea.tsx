import * as React from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "../../lib/utils";
import * as api from "../../api";

/**
 * Textarea with file autocomplete triggered by @ symbol.
 * When user types @ after whitespace, shows a dropdown of repository files.
 *
 * @example
 * // Single repository mode
 * <FileAutocompleteTextarea
 *   repositoryId="repo-123"
 *   onValueChange={(val) => setPrompt(val)}
 *   placeholder="Type @ to mention files"
 * />
 *
 * @example
 * // Repository group mode
 * <FileAutocompleteTextarea
 *   repositoryGroupId="group-123"
 *   onValueChange={(val) => setPrompt(val)}
 *   placeholder="Type @ to mention files"
 * />
 */

interface FileAutocompleteTextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  /** Single repository ID for autocomplete */
  repositoryId?: string;
  /** Repository group ID for autocomplete (takes precedence over repositoryId) */
  repositoryGroupId?: string;
  onValueChange?: (value: string) => void;
}

interface MentionState {
  isActive: boolean;
  startIndex: number;
  query: string;
}

const FileAutocompleteTextarea = React.forwardRef<
  HTMLTextAreaElement,
  FileAutocompleteTextareaProps
>(({ className, repositoryId, repositoryGroupId, onValueChange, value, onChange, ...props }, ref) => {
  const [internalValue, setInternalValue] = useState(
    (value as string) ?? ""
  );
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mentionState, setMentionState] = useState<MentionState>({
    isActive: false,
    startIndex: -1,
    query: "",
  });
  const [dropdownPosition, setDropdownPosition] = useState<{
    top: number;
    left: number;
  } | null>(null);

  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const dropdownRef = useRef<HTMLDivElement | null>(null);
  const blurTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debounceTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const requestIdRef = useRef(0);

  const actualValue = (value as string) ?? internalValue;

  // Cleanup timeouts on unmount
  useEffect(() => {
    return () => {
      if (blurTimeoutRef.current) {
        clearTimeout(blurTimeoutRef.current);
      }
      if (debounceTimeoutRef.current) {
        clearTimeout(debounceTimeoutRef.current);
      }
    };
  }, []);

  const setRef = useCallback(
    (element: HTMLTextAreaElement | null) => {
      textareaRef.current = element;
      if (typeof ref === "function") {
        ref(element);
      } else if (ref) {
        ref.current = element;
      }
    },
    [ref]
  );

  const fetchSuggestions = useCallback(
    async (query: string) => {
      // Repository group ID takes precedence over single repository ID
      if (!repositoryGroupId && !repositoryId) {
        setSuggestions([]);
        return;
      }

      // Increment request ID to track the latest request
      const currentRequestId = ++requestIdRef.current;

      setIsLoading(true);
      setError(null);

      try {
        const files = repositoryGroupId
          ? await api.listRepositoryGroupFiles(repositoryGroupId, query || undefined, 20)
          : await api.listRepositoryFiles(repositoryId!, query || undefined, 20);

        // Only update state if this is still the latest request
        if (currentRequestId === requestIdRef.current) {
          setSuggestions(files);
          setSelectedIndex(0);
          setIsLoading(false);
        }
      } catch (err) {
        // Only update state if this is still the latest request
        if (currentRequestId === requestIdRef.current) {
          console.error("Failed to fetch file suggestions:", err);
          setSuggestions([]);
          setError("Failed to load file suggestions");
          setIsLoading(false);
        }
      }
    },
    [repositoryId, repositoryGroupId]
  );

  // Debounced effect for fetching suggestions
  useEffect(() => {
    if (mentionState.isActive && (repositoryId || repositoryGroupId)) {
      // Clear any pending debounce timeout
      if (debounceTimeoutRef.current) {
        clearTimeout(debounceTimeoutRef.current);
      }

      // Debounce the API call by 200ms
      debounceTimeoutRef.current = setTimeout(() => {
        fetchSuggestions(mentionState.query);
      }, 200);
    } else {
      setSuggestions([]);
      setError(null);
      setIsLoading(false);
    }
  }, [mentionState.isActive, mentionState.query, repositoryId, repositoryGroupId, fetchSuggestions]);

  const calculateDropdownPosition = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea || !mentionState.isActive) {
      setDropdownPosition(null);
      return;
    }

    const text = actualValue.substring(0, mentionState.startIndex + 1);
    const lines = text.split("\n");
    const currentLineIndex = lines.length - 1;
    const currentLineText = lines[currentLineIndex];

    const computedStyle = window.getComputedStyle(textarea);
    const lineHeight = parseFloat(computedStyle.lineHeight) || 20;
    const paddingTop = parseFloat(computedStyle.paddingTop) || 0;
    const paddingLeft = parseFloat(computedStyle.paddingLeft) || 0;
    const fontSize = parseFloat(computedStyle.fontSize) || 14;

    // Estimate character width (monospace assumption, with fallback)
    const charWidth = fontSize * 0.6;

    // Calculate position
    const top = paddingTop + (currentLineIndex + 1) * lineHeight - textarea.scrollTop;
    const left = paddingLeft + currentLineText.length * charWidth;

    setDropdownPosition({ top, left: Math.min(left, textarea.clientWidth - 200) });
  }, [actualValue, mentionState.isActive, mentionState.startIndex]);

  useEffect(() => {
    calculateDropdownPosition();
  }, [calculateDropdownPosition]);

  const detectMention = useCallback(
    (text: string, cursorPosition: number) => {
      // Look backwards from cursor for @ symbol
      let atIndex = -1;
      for (let i = cursorPosition - 1; i >= 0; i--) {
        const char = text[i];
        if (char === "@") {
          atIndex = i;
          break;
        }
        // Stop if we hit whitespace or newline before finding @
        if (char === " " || char === "\n" || char === "\t") {
          break;
        }
      }

      if (atIndex >= 0) {
        // Check if @ is at start or preceded by whitespace
        const isValidStart =
          atIndex === 0 ||
          [" ", "\n", "\t"].includes(text[atIndex - 1]);

        if (isValidStart) {
          const query = text.substring(atIndex + 1, cursorPosition);
          // Only show if query doesn't contain spaces
          if (!query.includes(" ")) {
            return {
              isActive: true,
              startIndex: atIndex,
              query,
            };
          }
        }
      }

      return {
        isActive: false,
        startIndex: -1,
        query: "",
      };
    },
    []
  );

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      const newValue = e.target.value;
      const cursorPosition = e.target.selectionStart ?? 0;

      setInternalValue(newValue);
      onValueChange?.(newValue);
      onChange?.(e);

      const newMentionState = detectMention(newValue, cursorPosition);
      setMentionState(newMentionState);
    },
    [onChange, onValueChange, detectMention]
  );

  const insertSuggestion = useCallback(
    (suggestion: string) => {
      const textarea = textareaRef.current;
      if (!textarea || !mentionState.isActive) return;

      const beforeMention = actualValue.substring(0, mentionState.startIndex);
      const afterCursor = actualValue.substring(
        mentionState.startIndex + 1 + mentionState.query.length
      );

      const newValue = `${beforeMention}@${suggestion}${afterCursor}`;
      const newCursorPosition = mentionState.startIndex + 1 + suggestion.length;

      setInternalValue(newValue);
      onValueChange?.(newValue);

      // Reset mention state
      setMentionState({
        isActive: false,
        startIndex: -1,
        query: "",
      });
      setSuggestions([]);
      setError(null);

      // Set cursor position after React update
      requestAnimationFrame(() => {
        textarea.setSelectionRange(newCursorPosition, newCursorPosition);
        textarea.focus();
      });
    },
    [actualValue, mentionState, onValueChange]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (!mentionState.isActive || suggestions.length === 0) {
        props.onKeyDown?.(e);
        return;
      }

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((prev) =>
            prev < suggestions.length - 1 ? prev + 1 : 0
          );
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((prev) =>
            prev > 0 ? prev - 1 : suggestions.length - 1
          );
          break;
        case "Enter":
        case "Tab":
          e.preventDefault();
          insertSuggestion(suggestions[selectedIndex]);
          break;
        case "Escape":
          e.preventDefault();
          setMentionState({
            isActive: false,
            startIndex: -1,
            query: "",
          });
          setSuggestions([]);
          setError(null);
          break;
        default:
          props.onKeyDown?.(e);
      }
    },
    [mentionState.isActive, suggestions, selectedIndex, insertSuggestion, props]
  );

  const handleBlur = useCallback(
    (e: React.FocusEvent<HTMLTextAreaElement>) => {
      // Clear any existing blur timeout
      if (blurTimeoutRef.current) {
        clearTimeout(blurTimeoutRef.current);
      }

      // Delay hiding to allow click on suggestion
      blurTimeoutRef.current = setTimeout(() => {
        if (!dropdownRef.current?.contains(document.activeElement)) {
          setMentionState({
            isActive: false,
            startIndex: -1,
            query: "",
          });
          setSuggestions([]);
          setError(null);
        }
      }, 150);
      props.onBlur?.(e);
    },
    [props]
  );

  // Scroll selected item into view
  useEffect(() => {
    const dropdown = dropdownRef.current;
    if (!dropdown) return;

    const selectedElement = dropdown.querySelector(`[data-index="${selectedIndex}"]`) as HTMLElement;
    if (selectedElement) {
      selectedElement.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  const dropdownId = "file-autocomplete-dropdown";
  const getOptionId = (index: number) => `file-autocomplete-option-${index}`;

  return (
    <div className="relative">
      <textarea
        ref={setRef}
        className={cn(
          "flex min-h-[60px] w-full rounded-md border border-input bg-transparent px-3 py-2 text-base shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
          className
        )}
        value={actualValue}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onBlur={handleBlur}
        aria-expanded={mentionState.isActive && (suggestions.length > 0 || isLoading)}
        aria-haspopup="listbox"
        aria-controls={mentionState.isActive ? dropdownId : undefined}
        aria-activedescendant={
          mentionState.isActive && suggestions.length > 0
            ? getOptionId(selectedIndex)
            : undefined
        }
        {...props}
      />

      {mentionState.isActive && dropdownPosition && (
        <div
          ref={dropdownRef}
          id={dropdownId}
          role="listbox"
          aria-label="File suggestions"
          className="absolute z-50 w-72 max-h-60 overflow-auto rounded-md border bg-popover shadow-lg"
          style={{
            top: dropdownPosition.top,
            left: dropdownPosition.left,
          }}
        >
          {isLoading && suggestions.length === 0 && (
            <div className="px-3 py-2 text-sm text-muted-foreground">
              Loading...
            </div>
          )}

          {error && (
            <div className="px-3 py-2 text-sm text-destructive">
              {error}
            </div>
          )}

          {!isLoading && !error && suggestions.length === 0 && (
            <div className="px-3 py-2 text-sm text-muted-foreground">
              No files found
            </div>
          )}

          {suggestions.map((file, index) => (
            <button
              key={file}
              id={getOptionId(index)}
              data-index={index}
              type="button"
              role="option"
              aria-selected={index === selectedIndex}
              className={cn(
                "flex w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-accent",
                index === selectedIndex && "bg-accent"
              )}
              onMouseDown={(e) => {
                e.preventDefault();
                insertSuggestion(file);
              }}
              onMouseEnter={() => setSelectedIndex(index)}
            >
              <FileIcon className="h-4 w-4 shrink-0 text-muted-foreground" />
              <span className="truncate">{file}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
});

FileAutocompleteTextarea.displayName = "FileAutocompleteTextarea";

function FileIcon({ className }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
      <polyline points="14 2 14 8 20 8" />
    </svg>
  );
}

export { FileAutocompleteTextarea };
