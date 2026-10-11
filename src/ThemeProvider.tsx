import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  initialThemeState,
  themeStyle,
  validateThemeState,
  type ThemeState,
} from "./themes";

interface Context {
  state: ThemeState;
  error: string;
  commit: (s: ThemeState) => Promise<void>;
}
const ThemeContext = createContext<Context>({
  state: initialThemeState(),
  error: "",
  commit: async () => {
    throw Error("Theme service is unavailable.");
  },
});
export const useThemes = () => useContext(ThemeContext);
export function useReducedMotion() {
  const [reduced, set] = useState(
    () =>
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false,
  );
  useEffect(() => {
    const m = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    const change = () => set(m?.matches ?? false);
    m?.addEventListener("change", change);
    return () => m?.removeEventListener("change", change);
  }, []);
  return reduced;
}
export default function ThemeProvider({ children }: { children: ReactNode }) {
  const [state, set] = useState(initialThemeState);
  const [error, setError] = useState("");
  const reduced = useReducedMotion();
  useEffect(() => {
    let live = true;
    const accept = (value: unknown) => {
      const s = validateThemeState(value);
      if (live) {
        set((old) => (s.revision >= old.revision ? s : old));
        setError("");
      }
    };
    const subscription = listen<ThemeState>("theme-state", (e) => {
      try {
        accept(e.payload);
      } catch (e) {
        setError(String(e));
      }
    });
    void subscription
      .then(() => invoke("theme_load"))
      .then(accept)
      .catch((e) => {
        if (live) setError(String(e));
      });
    const visibility = () => {
      document.documentElement.dataset.hidden = String(document.hidden);
    };
    const visible = listen<boolean>("ui-visible", (e) => {
      document.documentElement.dataset.hidden = String(
        !e.payload || document.hidden,
      );
    }).catch(() => () => {});
    void visible
      .then(() => invoke<boolean>("presentation_visibility"))
      .then((shown) => {
        if (live)
          document.documentElement.dataset.hidden = String(
            !shown || document.hidden,
          );
      })
      .catch(() => {});
    document.addEventListener("visibilitychange", visibility);
    visibility();
    return () => {
      live = false;
      void subscription.then((stop) => stop()).catch(() => {});
      void visible.then((stop) => stop()).catch(() => {});
      document.removeEventListener("visibilitychange", visibility);
    };
  }, []);
  useEffect(() => {
    const style = themeStyle(
      state.active,
      reduced,
      new URLSearchParams(location.search).has("mini") ? 0.65 : 1,
    );
    for (const [k, v] of Object.entries(style))
      document.documentElement.style.setProperty(k, String(v));
  }, [state.active, reduced]);
  const commit = async (next: ThemeState) => {
    const result = validateThemeState(
      await invoke("theme_save", { state: validateThemeState(next) }),
    );
    set((old) => (result.revision >= old.revision ? result : old));
    setError("");
  };
  return (
    <ThemeContext.Provider value={{ state, error, commit }}>
      {children}
    </ThemeContext.Provider>
  );
}
