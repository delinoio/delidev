import * as React from "react";
import { cn } from "../../lib/utils";

interface PopoverContextValue {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  triggerRef: React.RefObject<HTMLButtonElement | null>;
}

const PopoverContext = React.createContext<PopoverContextValue | null>(null);

interface PopoverProps {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  children: React.ReactNode;
}

function Popover({ open, onOpenChange, children }: PopoverProps) {
  const [internalOpen, setInternalOpen] = React.useState(false);
  const triggerRef = React.useRef<HTMLButtonElement | null>(null);

  const isControlled = onOpenChange !== undefined;
  const isOpen = isControlled ? (open ?? false) : internalOpen;
  const setIsOpen = isControlled ? onOpenChange : setInternalOpen;

  return (
    <PopoverContext.Provider value={{ open: isOpen, onOpenChange: setIsOpen, triggerRef }}>
      {children}
    </PopoverContext.Provider>
  );
}

function usePopoverContext() {
  const context = React.useContext(PopoverContext);
  if (!context) {
    throw new Error("Popover components must be used within a Popover");
  }
  return context;
}

interface PopoverTriggerProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  asChild?: boolean;
}

const PopoverTrigger = React.forwardRef<HTMLButtonElement, PopoverTriggerProps>(
  ({ children, asChild, onClick, ...props }, forwardedRef) => {
    const { onOpenChange, open, triggerRef } = usePopoverContext();

    const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
      onClick?.(e);
      onOpenChange(!open);
    };

    // Combine forwarded ref with internal ref
    const ref = React.useCallback(
      (node: HTMLButtonElement | null) => {
        triggerRef.current = node;
        if (typeof forwardedRef === "function") {
          forwardedRef(node);
        } else if (forwardedRef) {
          forwardedRef.current = node;
        }
      },
      [forwardedRef, triggerRef]
    );

    if (asChild && React.isValidElement(children)) {
      return React.cloneElement(children as React.ReactElement<{
        onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
        ref?: React.Ref<HTMLButtonElement>;
      }>, {
        onClick: handleClick,
        ref,
        ...props,
      });
    }

    return (
      <button type="button" ref={ref} onClick={handleClick} {...props}>
        {children}
      </button>
    );
  }
);
PopoverTrigger.displayName = "PopoverTrigger";

interface PopoverContentProps extends React.HTMLAttributes<HTMLDivElement> {
  align?: "start" | "center" | "end";
  sideOffset?: number;
}

function PopoverContent({
  children,
  className,
  align = "start",
  sideOffset = 4,
  ...props
}: PopoverContentProps) {
  const { open, onOpenChange, triggerRef } = usePopoverContext();
  const contentRef = React.useRef<HTMLDivElement>(null);

  // Close on click outside
  React.useEffect(() => {
    if (!open) return;

    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      if (
        contentRef.current &&
        !contentRef.current.contains(target) &&
        triggerRef.current &&
        !triggerRef.current.contains(target)
      ) {
        onOpenChange(false);
      }
    };

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onOpenChange(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [open, onOpenChange, triggerRef]);

  if (!open) return null;

  const alignmentClasses = {
    start: "left-0",
    center: "left-1/2 -translate-x-1/2",
    end: "right-0",
  };

  return (
    <div
      ref={contentRef}
      className={cn(
        "absolute z-50 min-w-[8rem] overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
        "animate-in fade-in-0 zoom-in-95",
        alignmentClasses[align],
        className
      )}
      style={{ marginTop: sideOffset }}
      {...props}
    >
      {children}
    </div>
  );
}

export { Popover, PopoverTrigger, PopoverContent };
