import { useEffect, useRef } from "react";
import {
  buildSnapshot,
  defaultProjectSnapshot,
  hydrateCache,
  loadProjectSnapshot,
  persistCache,
  saveProjectSnapshot,
} from "@/domains/chat/project-cache";
import { useAppStore } from "@/store";

/**
 * Orchestrates saving/restoring chat state when the user switches projects.
 *
 * - Saves the outgoing project's snapshot (chat state + settings + draft)
 * - Disconnects the SSE for the outgoing project
 * - Restores the incoming project's snapshot (or creates a default one)
 * - Reconnects SSE if the incoming project was mid-stream
 */
export function useProjectSwitch(
  disconnect: () => void,
  connectToSession: (sessionId: string, turnId: string) => void,
) {
  const selectedProject = useAppStore((s) => s.selectedProject);
  const prevProjectRef = useRef<string | null>(null);
  const hydratedRef = useRef(false);

  // Hydrate cache from localStorage on first mount
  useEffect(() => {
    if (!hydratedRef.current) {
      hydrateCache();
      hydratedRef.current = true;
    }
  }, []);

  // Save current + persist on unload
  useEffect(() => {
    function handleBeforeUnload() {
      const state = useAppStore.getState();
      const project = state.selectedProject;
      if (project) {
        saveProjectSnapshot(project, buildSnapshot(state));
      }
      persistCache();
    }
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, []);

  // Core switch logic
  useEffect(() => {
    // Skip initial mount — nothing to save yet
    if (prevProjectRef.current === null && selectedProject !== null) {
      prevProjectRef.current = selectedProject;
      // On first mount, try to restore from cache if we have one
      const cached = loadProjectSnapshot(selectedProject);
      if (cached) {
        restoreFromSnapshot(cached, disconnect, connectToSession);
      }
      return;
    }

    // No change
    if (selectedProject === prevProjectRef.current) return;

    const outgoingProject = prevProjectRef.current;
    prevProjectRef.current = selectedProject;

    // Save outgoing project state
    if (outgoingProject) {
      const state = useAppStore.getState();
      saveProjectSnapshot(outgoingProject, buildSnapshot(state));
    }

    // Disconnect SSE from outgoing project
    disconnect();

    // Restore incoming project
    if (!selectedProject) return;

    const snapshot = loadProjectSnapshot(selectedProject);
    if (snapshot) {
      restoreFromSnapshot(snapshot, disconnect, connectToSession);
    } else {
      // No cached state — reset to defaults
      const defaults = defaultProjectSnapshot();
      const store = useAppStore.getState();
      store.restoreChat(defaults);
      store.setSelectedModel(defaults.selectedModel);
      store.setEffort(defaults.effort);
      store.setChatMode(defaults.chatMode);
    }
  }, [selectedProject, disconnect, connectToSession]);
}

function restoreFromSnapshot(
  snapshot: ReturnType<typeof loadProjectSnapshot> & object,
  _disconnect: () => void,
  connectToSession: (sessionId: string, turnId: string) => void,
) {
  const store = useAppStore.getState();
  store.restoreChat(snapshot);
  store.setSelectedModel(snapshot.selectedModel);
  store.setEffort(snapshot.effort);
  store.setChatMode(snapshot.chatMode);

  // Reconnect SSE if the project was mid-stream
  if (snapshot.wasStreaming && snapshot.streamingSessionId && snapshot.streamingTurnId) {
    connectToSession(snapshot.streamingSessionId, snapshot.streamingTurnId);
  }
}
