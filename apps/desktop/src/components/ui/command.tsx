import * as React from "react";
import { cn } from "../../lib/utils";
import { Search } from "lucide-react";

interface CommandContextValue {
  search: string;
  setSearch: (search: string) => void;
}

const CommandContext = React.createContext<CommandContextValue | null>(null);

interface CommandProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
}

function Command({ children, className, ...props }: CommandProps) {
  const [search, setSearch] = React.useState("");

  return (
    <CommandContext.Provider value={{ search, setSearch }}>
      <div
        className={cn(
          "flex h-full w-full flex-col overflow-hidden rounded-md bg-popover text-popover-foreground",
          className
        )}
        {...props}
      >
        {children}
      </div>
    </CommandContext.Provider>
  );
}

function useCommandContext() {
  const context = React.useContext(CommandContext);
  if (!context) {
    throw new Error("Command components must be used within a Command");
  }
  return context;
}

interface CommandInputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "onChange"> {
  onValueChange?: (value: string) => void;
}

const CommandInput = React.forwardRef<HTMLInputElement, CommandInputProps>(
  ({ className, onValueChange, ...props }, ref) => {
    const { search, setSearch } = useCommandContext();

    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.value;
      setSearch(value);
      onValueChange?.(value);
    };

    return (
      <div className="flex items-center border-b px-3">
        <Search className="mr-2 h-4 w-4 shrink-0 opacity-50" />
        <input
          ref={ref}
          value={search}
          onChange={handleChange}
          className={cn(
            "flex h-10 w-full rounded-md bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50",
            className
          )}
          {...props}
        />
      </div>
    );
  }
);
CommandInput.displayName = "CommandInput";

interface CommandListProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
}

function CommandList({ children, className, ...props }: CommandListProps) {
  return (
    <div
      className={cn("max-h-[300px] overflow-y-auto overflow-x-hidden", className)}
      {...props}
    >
      {children}
    </div>
  );
}

interface CommandEmptyProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
}

function CommandEmpty({ children, className, ...props }: CommandEmptyProps) {
  return (
    <div className={cn("py-6 text-center text-sm", className)} {...props}>
      {children}
    </div>
  );
}

interface CommandGroupProps extends React.HTMLAttributes<HTMLDivElement> {
  heading?: string;
  children: React.ReactNode;
}

function CommandGroup({ heading, children, className, ...props }: CommandGroupProps) {
  return (
    <div
      className={cn(
        "overflow-hidden p-1 text-foreground",
        className
      )}
      {...props}
    >
      {heading && (
        <div className="px-2 py-1.5 text-xs font-medium text-muted-foreground">
          {heading}
        </div>
      )}
      {children}
    </div>
  );
}

interface CommandItemProps extends React.HTMLAttributes<HTMLDivElement> {
  value?: string;
  onSelect?: () => void;
  disabled?: boolean;
  children: React.ReactNode;
}

const CommandItem = React.forwardRef<HTMLDivElement, CommandItemProps>(
  ({ children, className, value, onSelect, disabled, ...props }, ref) => {
    const { search } = useCommandContext();

    // Filter based on search
    const isFiltered = value && search && !value.toLowerCase().includes(search.toLowerCase());
    if (isFiltered) return null;

    return (
      <div
        ref={ref}
        className={cn(
          "relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none",
          "hover:bg-accent hover:text-accent-foreground",
          "focus:bg-accent focus:text-accent-foreground",
          disabled && "pointer-events-none opacity-50",
          className
        )}
        onClick={() => !disabled && onSelect?.()}
        role="option"
        aria-selected={false}
        {...props}
      >
        {children}
      </div>
    );
  }
);
CommandItem.displayName = "CommandItem";

interface CommandSeparatorProps extends React.HTMLAttributes<HTMLDivElement> {}

function CommandSeparator({ className, ...props }: CommandSeparatorProps) {
  return (
    <div className={cn("-mx-1 h-px bg-border", className)} {...props} />
  );
}

export {
  Command,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandSeparator,
};
