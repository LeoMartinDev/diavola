type Theme = "auto" | "light" | "dark";

const STORAGE_KEY = "diavola-theme";

function resolveTheme(theme: Theme, systemDark: boolean): "light" | "dark" {
  if (theme === "auto") return systemDark ? "dark" : "light";
  return theme;
}

function applyDataTheme(resolved: "light" | "dark") {
  document.documentElement.setAttribute("data-theme", resolved);
}

class ThemeStore {
  theme = $state<Theme>("auto");
  resolved = $state<"light" | "dark">("dark");

  #mediaQuery: MediaQueryList | null = null;
  #mediaHandler: ((e: MediaQueryListEvent) => void) | null = null;

  constructor() {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "auto") {
      this.theme = stored;
    }
    this.resolved = resolveTheme(this.theme, this.#systemDark());
    applyDataTheme(this.resolved);

    if (this.theme === "auto") {
      this.#listen();
    }
  }

  setTheme(theme: Theme) {
    this.theme = theme;
    localStorage.setItem(STORAGE_KEY, theme);

    if (theme === "auto") {
      this.#listen();
      this.resolved = resolveTheme("auto", this.#systemDark());
    } else {
      this.#unlisten();
      this.resolved = theme;
    }

    applyDataTheme(this.resolved);
  }

  #systemDark(): boolean {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  }

  #listen() {
    if (this.#mediaQuery) return;
    this.#mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    this.#mediaHandler = (e: MediaQueryListEvent) => {
      if (this.theme === "auto") {
        this.resolved = e.matches ? "dark" : "light";
        applyDataTheme(this.resolved);
      }
    };
    this.#mediaQuery.addEventListener("change", this.#mediaHandler);
  }

  #unlisten() {
    if (this.#mediaQuery && this.#mediaHandler) {
      this.#mediaQuery.removeEventListener("change", this.#mediaHandler);
      this.#mediaQuery = null;
      this.#mediaHandler = null;
    }
  }
}

export const themeStore = new ThemeStore();
