import { useState, useEffect } from "react";
import { useDrones } from "@/hooks/use-drones";
import { useTheme } from "@/hooks/use-theme";
import { Sidebar } from "@/components/sidebar";
import { DetailPanel } from "@/components/detail-panel";

export default function App() {
  const { drones, connectionStatus } = useDrones();
  const [selectedDrone, setSelectedDrone] = useState<string | null>(null);
  useTheme(); // Apply theme on mount

  const isMock = connectionStatus === "mock";

  // Auto-select first drone if only one
  useEffect(() => {
    if (drones.length === 1 && !selectedDrone) {
      setSelectedDrone(drones[0]!.name);
    }
  }, [drones, selectedDrone]);

  // Auto-select first drone in mock mode
  useEffect(() => {
    if (isMock && drones.length > 0 && !selectedDrone) {
      setSelectedDrone(drones[0]!.name);
    }
  }, [isMock, drones, selectedDrone]);

  const activeDrone = drones.find((d) => d.name === selectedDrone) ?? null;

  return (
    <div className="flex h-screen">
      <Sidebar
        drones={drones}
        selectedDrone={selectedDrone}
        onSelectDrone={setSelectedDrone}
        connectionStatus={connectionStatus}
      />
      <DetailPanel drone={activeDrone} isMock={isMock} />
    </div>
  );
}
