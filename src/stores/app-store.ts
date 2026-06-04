import { create } from "zustand";

/**
 * App-wide UI state shared across the Glossary / Translate / Verify screens.
 * Persisted config (connections, defaults) lives in the Tauri store on the Rust
 * side; this holds the current session's working selections.
 */
interface AppState {
  workdir: string | null;
  sourceLang: string;
  targetLang: string;
  activeConnection: string | null;
  setWorkdir: (dir: string | null) => void;
  setLanguages: (source: string, target: string) => void;
  setActiveConnection: (name: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  workdir: null,
  sourceLang: "zh",
  targetLang: "en",
  activeConnection: null,
  setWorkdir: (workdir) => set({ workdir }),
  setLanguages: (sourceLang, targetLang) => set({ sourceLang, targetLang }),
  setActiveConnection: (activeConnection) => set({ activeConnection }),
}));
