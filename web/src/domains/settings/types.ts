// ── Auth types ───────────────────────────────────────────────────────────────

export type ProviderType = "anthropic" | "bedrock";

export interface AuthStatus {
  configured: boolean;
  type: "api_key" | "oauth" | "bedrock" | null;
  expired: boolean;
  profile?: string;
  provider?: ProviderType;
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

// ── Profile types ───────────────────────────────────────────────────────────

export interface ProfileInfo {
  name: string;
  description?: string;
  provider: ProviderType;
  is_active: boolean;
  has_credentials: boolean;
}

export interface ActiveProfile {
  name: string;
  provider: ProviderType;
}

export interface CreateProfileParams {
  name: string;
  description?: string;
  provider: ProviderType;
  region?: string;
  access_key_id?: string;
  secret_access_key?: string;
  session_token?: string;
  api_key?: string;
  aws_profile?: string;
}

export interface AwsProfileInfo {
  name: string;
  region?: string;
  sso_start_url?: string;
}
