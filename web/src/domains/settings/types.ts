// ── Auth types ───────────────────────────────────────────────────────────────

export interface AuthStatus {
  configured: boolean;
  type: "api_key" | "oauth" | null;
  expired: boolean;
}

export interface Model {
  id: string;
  name: string;
  description: string;
}

export interface OAuthAuthorizeResponse {
  authorize_url: string;
  state: string;
}
