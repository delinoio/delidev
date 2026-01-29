import * as Sentry from "@sentry/react";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/globals.css";

const SENTRY_DSN =
  "https://9930ad2c1205512beb8738957c0a0271@o4510703761227776.ingest.us.sentry.io/4510703764635648";

Sentry.init({
  dsn: SENTRY_DSN,
  release: `delidev@${import.meta.env.PUBLIC_APP_VERSION || "0.0.1"}`,
  integrations: [
    Sentry.browserTracingIntegration(),
    Sentry.replayIntegration({
      maskAllText: true,
      maskAllInputs: true,
      blockAllMedia: true,
    }),
  ],
  tracesSampleRate: 0.1,
  replaysSessionSampleRate: 0.1,
  replaysOnErrorSampleRate: 1.0,
});

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element not found. Ensure index.html contains a div with id='root'");
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
