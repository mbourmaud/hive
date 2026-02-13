export const queryKeys = {
  auth: {
    status: () => ["auth", "status"] as const,
    models: () => ["auth", "models"] as const,
  },
  sessions: {
    all: () => ["sessions"] as const,
    list: () => ["sessions", "list"] as const,
    detail: (id: string) => ["sessions", "detail", id] as const,
    history: (id: string) => ["sessions", "history", id] as const,
  },
  projects: {
    all: () => ["projects"] as const,
  },
} as const;
