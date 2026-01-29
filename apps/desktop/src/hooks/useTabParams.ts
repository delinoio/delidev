import { useContext } from "react";
import { useParams } from "react-router-dom";
import { TabParamsContext } from "../components/layout/TabContent";

/**
 * Hook to get route params for the current tab.
 *
 * When used within TabContent, this returns params extracted from the tab's stored path,
 * allowing state preservation when switching between tabs.
 *
 * When used outside of TabContent (e.g., during initial render or in non-tabbed contexts),
 * this falls back to React Router's useParams().
 *
 * @returns Route parameters object (same shape as useParams)
 */
export function useTabParams<
  T extends Record<string, string | undefined> = Record<string, string | undefined>
>(): T {
  const context = useContext(TabParamsContext);
  const routerParams = useParams();

  // If we're within TabContent context, use the tab's params
  if (context) {
    return context.params as T;
  }

  // Fallback to React Router's params
  return routerParams as T;
}
