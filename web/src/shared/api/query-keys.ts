export const queryKeys = {
  auth: {
    status: () => ["auth", "status"] as const,
    models: () => ["auth", "models"] as const,
  },
  profiles: {
    all: () => ["profiles"] as const,
    active: () => ["profiles", "active"] as const,
  },
  aws: {
    profiles: () => ["aws", "profiles"] as const,
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
  registry: {
    all: () => ["registry"] as const,
    detail: (id: string) => ["registry", "detail", id] as const,
  },
  status: {
    all: () => ["status"] as const,
  },
} as const;
