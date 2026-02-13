type InvokeFunction = (cmd: string, args: Record<string, unknown>) => Promise<unknown>;

function getTauriInvoke(): InvokeFunction | null {
  const w = window as Record<string, unknown>;
  if (w.__TAURI_INTERNALS__) {
    return (w.__TAURI_INTERNALS__ as { invoke: InvokeFunction }).invoke;
  }
  return null;
}

export function useActions() {
  const invoke = getTauriInvoke();
  const isTauri = invoke !== null;

  const stopDrone = async (name: string): Promise<string> => {
    if (invoke) {
      return invoke("stop_drone", { name }) as Promise<string>;
    }
    throw new Error("Stop drone is only available in the desktop app");
  };

  const cleanDrone = async (name: string): Promise<string> => {
    if (invoke) {
      return invoke("clean_drone", { name }) as Promise<string>;
    }
    throw new Error("Clean drone is only available in the desktop app");
  };

  const startDrone = async (name: string, plan: string, model: string, mode: string): Promise<string> => {
    if (invoke) {
      return invoke("start_drone", { name, plan, model, mode }) as Promise<string>;
    }
    throw new Error("Start drone is only available in the desktop app");
  };

  return { isTauri, stopDrone, cleanDrone, startDrone };
}
