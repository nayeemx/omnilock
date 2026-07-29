import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

class ErrorBoundary extends React.Component<{children: React.ReactNode}, {hasError: boolean}> {
  constructor(props: {children: React.ReactNode}) {
    super(props);
    this.state = {hasError: false};
  }
  static getDerivedStateFromError() {
    return {hasError: true};
  }
  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("React ErrorBoundary caught:", error, info);
  }
  render() {
    if (this.state.hasError) {
      return (
        <div className="flex items-center justify-center h-screen" style={{background: "var(--background)"}}>
          <div className="text-center p-8">
            <img src="/icon.png" alt="OmniLock" className="w-12 h-12 mx-auto mb-4 rounded-xl object-cover" />
            <h2 className="text-xl font-semibold mb-2">Something went wrong</h2>
            <p className="text-sm text-[color:var(--muted-foreground)] mb-4">An unexpected error occurred.</p>
            <button onClick={() => window.location.reload()}
                    className="px-4 py-2 rounded-lg text-sm font-medium text-[color:var(--primary-foreground)] glow-cyan"
                    style={{background: "var(--gradient-brand)"}}>
              Reload
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

function applyTheme() {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");
  const html = document.documentElement;

  function update(dark: boolean) {
    if (dark) {
      html.classList.add("dark");
      html.classList.remove("light");
    } else {
      html.classList.add("light");
      html.classList.remove("dark");
    }
  }

  update(prefersDark.matches);
  prefersDark.addEventListener("change", (e) => update(e.matches));
}

applyTheme();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>
);
