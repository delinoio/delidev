import { describe, it, expect } from "vitest";
import { cn } from "./utils";

describe("cn utility function", () => {
  it("should merge class names", () => {
    expect(cn("foo", "bar")).toBe("foo bar");
  });

  it("should handle empty inputs", () => {
    expect(cn()).toBe("");
  });

  it("should handle undefined and null values", () => {
    expect(cn("foo", undefined, "bar", null)).toBe("foo bar");
  });

  it("should handle conditional classes with objects", () => {
    expect(cn("base", { active: true, disabled: false })).toBe("base active");
  });

  it("should handle arrays of classes", () => {
    expect(cn(["foo", "bar"], "baz")).toBe("foo bar baz");
  });

  it("should merge conflicting Tailwind classes correctly", () => {
    expect(cn("px-2", "px-4")).toBe("px-4");
    expect(cn("text-red-500", "text-blue-500")).toBe("text-blue-500");
  });

  it("should handle complex Tailwind class combinations", () => {
    expect(cn("p-4 bg-red-500", "p-2 text-white")).toBe(
      "bg-red-500 p-2 text-white"
    );
  });

  it("should preserve non-conflicting classes", () => {
    expect(cn("px-4 py-2", "mt-4 mb-2")).toBe("px-4 py-2 mt-4 mb-2");
  });

  it("should handle boolean false values", () => {
    expect(cn("foo", false && "bar", "baz")).toBe("foo baz");
  });

  it("should handle mixed conditions", () => {
    const isActive = true;
    const isDisabled = false;
    expect(
      cn("base", isActive && "active", isDisabled && "disabled", {
        focused: true,
        hidden: false,
      })
    ).toBe("base active focused");
  });
});
