import { useEffect, useState, lazy, Suspense } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { Toaster } from "sonner";
import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";
import { Layout } from "./components/layout/Layout";
import { useAppStore } from "./stores/app";
import { useRepositoriesStore } from "./stores/repositories";
import { Loader2 } from "lucide-react";
import { useNotificationClickHandler } from "./hooks/useNotificationClickHandler";
import { hasModeBeenSelected, clearModeSelection } from "./pages/ModeSelection";

// Lazy-loaded page components for code splitting
const Dashboard = lazy(() => import("./pages/Dashboard").then(m => ({ default: m.Dashboard })));
const Repositories = lazy(() => import("./pages/Repositories").then(m => ({ default: m.Repositories })));
const RepositoryGroups = lazy(() => import("./pages/RepositoryGroups").then(m => ({ default: m.RepositoryGroups })));
const Settings = lazy(() => import("./pages/Settings").then(m => ({ default: m.Settings })));
const RepositorySettings = lazy(() => import("./pages/RepositorySettings").then(m => ({ default: m.RepositorySettings })));
const Chat = lazy(() => import("./pages/Chat").then(m => ({ default: m.Chat })));
const UnitTaskDetail = lazy(() => import("./pages/UnitTaskDetail").then(m => ({ default: m.UnitTaskDetail })));
const CompositeTaskDetail = lazy(() => import("./pages/CompositeTaskDetail").then(m => ({ default: m.CompositeTaskDetail })));
const Onboarding = lazy(() => import("./pages/Onboarding").then(m => ({ default: m.Onboarding })));
const ModeSelection = lazy(() => import("./pages/ModeSelection").then(m => ({ default: m.ModeSelection })));

function AppContent() {
  const { isInitialized, isLoading, initialize } = useAppStore();
  const { repositories, hasFetched, fetchRepositories } = useRepositoriesStore();
  const [modeSelected, setModeSelected] = useState<boolean | null>(null);

  // Check if mode has been selected (or if dev mode forces re-selection)
  useEffect(() => {
    // Check for force mode selection query param (works in dev and prod)
    const urlParams = new URLSearchParams(window.location.search);
    const forceSelection = urlParams.get("force_mode_selection") === "true";

    if (forceSelection) {
      clearModeSelection();
      // Clean up the URL
      window.history.replaceState({}, document.title, window.location.pathname);
      setModeSelected(false);
      return;
    }

    // In dev mode, always show mode selection on each start
    // This allows developers to test both Local and Server modes easily
    if (import.meta.env.DEV) {
      clearModeSelection();
      setModeSelected(false);
      return;
    }

    setModeSelected(hasModeBeenSelected());
  }, []);

  useEffect(() => {
    initialize();
  }, [initialize]);

  useEffect(() => {
    if (isInitialized) {
      fetchRepositories();
    }
  }, [isInitialized, fetchRepositories]);

  // Request notification permission on startup
  useEffect(() => {
    async function checkAndRequestNotificationPermission() {
      try {
        const granted = await isPermissionGranted();
        if (!granted) {
          const permission = await requestPermission();
          if (permission === "granted") {
            console.log("Notification permission granted");
          }
        }
      } catch (error) {
        console.error("Failed to request notification permission:", error);
      }
    }

    checkAndRequestNotificationPermission();
  }, []);

  // Still checking mode selection status
  if (modeSelected === null) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          <p className="text-sm text-muted-foreground">Loading DeliDev...</p>
        </div>
      </div>
    );
  }

  if (isLoading && !isInitialized) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          <p className="text-sm text-muted-foreground">Loading DeliDev...</p>
        </div>
      </div>
    );
  }

  // Wait for repositories to be fetched before deciding on onboarding
  if (isInitialized && !hasFetched) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          <p className="text-sm text-muted-foreground">Loading DeliDev...</p>
        </div>
      </div>
    );
  }

  // Show mode selection if mode hasn't been selected yet
  const needsModeSelection = !modeSelected;

  // Show onboarding if no repositories are registered
  const needsOnboarding = isInitialized && hasFetched && repositories.length === 0;

  // Fallback component for lazy-loaded pages
  const PageLoadingFallback = (
    <div className="flex-1 flex items-center justify-center">
      <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
    </div>
  );

  return (
    <BrowserRouter>
      <NotificationHandler />
      <Suspense fallback={PageLoadingFallback}>
        <Routes>
          <Route path="/mode-selection" element={<ModeSelection />} />
          <Route path="/onboarding" element={<Onboarding />} />
          {needsModeSelection ? (
            <Route path="*" element={<Navigate to="/mode-selection" replace />} />
          ) : needsOnboarding ? (
            <Route path="*" element={<Navigate to="/onboarding" replace />} />
          ) : (
            <Route element={<Layout />}>
              <Route path="/" element={<Dashboard />} />
              <Route path="/repositories" element={<Repositories />} />
              <Route path="/repository-groups" element={<RepositoryGroups />} />
              <Route path="/settings" element={<Navigate to="/settings/global" replace />} />
              <Route path="/settings/:tab" element={<Settings />} />
              <Route
                path="/settings/repository/:id"
                element={<RepositorySettings />}
              />
              <Route path="/chat" element={<Chat />} />
              <Route path="/unit-tasks/:id" element={<UnitTaskDetail />} />
              <Route
                path="/composite-tasks/:id"
                element={<CompositeTaskDetail />}
              />
            </Route>
          )}
        </Routes>
      </Suspense>
    </BrowserRouter>
  );
}

/**
 * Component that sets up notification click handlers.
 * Must be rendered inside BrowserRouter to access useNavigate.
 */
function NotificationHandler() {
  useNotificationClickHandler();
  return null;
}

function App() {
  return (
    <>
      <AppContent />
      <Toaster position="top-right" richColors />
    </>
  );
}

export default App;
