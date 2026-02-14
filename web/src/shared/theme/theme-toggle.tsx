import { Moon, Sun } from "lucide-react";
import { useTheme } from "@/shared/theme/use-theme";
import { Button } from "@/shared/ui/button";

export function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();
  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={toggleTheme}
      title="Toggle light/dark theme"
      className="h-8 w-8"
    >
      {theme === "dark" ? <Moon className="h-4 w-4" /> : <Sun className="h-4 w-4" />}
    </Button>
  );
}
