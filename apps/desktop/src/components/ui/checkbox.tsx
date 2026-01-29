import * as React from "react";
import { cn } from "../../lib/utils";
import { Check } from "lucide-react";

interface CheckboxProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "type"> {
  onCheckedChange?: (checked: boolean) => void;
}

const Checkbox = React.forwardRef<HTMLInputElement, CheckboxProps>(
  ({ className, onCheckedChange, onChange, ...props }, ref) => {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange?.(e);
      onCheckedChange?.(e.target.checked);
    };

    return (
      <label className="relative inline-flex items-center cursor-pointer">
        <input
          type="checkbox"
          ref={ref}
          className="sr-only peer"
          onChange={handleChange}
          {...props}
        />
        <div
          className={cn(
            "h-4 w-4 shrink-0 rounded-sm border border-primary shadow peer-focus-visible:ring-1 peer-focus-visible:ring-ring peer-disabled:cursor-not-allowed peer-disabled:opacity-50 peer-checked:bg-primary peer-checked:text-primary-foreground flex items-center justify-center",
            className
          )}
        >
          <Check className="h-3 w-3 text-primary-foreground opacity-0 peer-checked:opacity-100 hidden peer-checked:flex" />
        </div>
      </label>
    );
  }
);
Checkbox.displayName = "Checkbox";

// Simpler version with visible check
const SimpleCheckbox = React.forwardRef<HTMLInputElement, CheckboxProps>(
  ({ className, checked, onCheckedChange, onChange, ...props }, ref) => {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange?.(e);
      onCheckedChange?.(e.target.checked);
    };

    return (
      <div
        className={cn(
          "h-4 w-4 shrink-0 rounded-sm border border-primary shadow focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 flex items-center justify-center cursor-pointer",
          checked && "bg-primary",
          className
        )}
        onClick={() => onCheckedChange?.(!checked)}
      >
        {checked && <Check className="h-3 w-3 text-primary-foreground" />}
        <input
          type="checkbox"
          ref={ref}
          className="sr-only"
          checked={checked}
          onChange={handleChange}
          {...props}
        />
      </div>
    );
  }
);
SimpleCheckbox.displayName = "SimpleCheckbox";

export { Checkbox, SimpleCheckbox };
