import { ExternalLink, Key } from "lucide-react";
import type { OAuthAuthorizeResponse } from "@/domains/settings/types";
import type { AnthropicAuthMethod } from "./profile-form-fields";

// ── Anthropic Fields ─────────────────────────────────────────────────────────

interface AnthropicFieldsProps {
  authMethod: AnthropicAuthMethod;
  setAuthMethod: (v: AnthropicAuthMethod) => void;
  apiKey: string;
  setApiKey: (v: string) => void;
  oauthData: OAuthAuthorizeResponse | null;
  oauthCode: string;
  setOauthCode: (v: string) => void;
  oauthVerified: boolean;
  loading: boolean;
  onStartOAuth: () => void;
  onVerifyOAuth: () => void;
}

export function AnthropicFields({
  authMethod, setAuthMethod,
  apiKey, setApiKey,
  oauthData, oauthCode, setOauthCode, oauthVerified,
  loading,
  onStartOAuth, onVerifyOAuth,
}: AnthropicFieldsProps) {
  return (
    <>
      <div data-slot="profile-field">
        <span data-slot="profile-label">Auth method</span>
        <div data-slot="profile-provider-selector">
          <button
            type="button"
            data-slot="profile-provider-btn"
            data-active={authMethod === "oauth" || undefined}
            onClick={() => setAuthMethod("oauth")}
          >
            <ExternalLink className="h-3.5 w-3.5" />
            Claude Pro/Max
          </button>
          <button
            type="button"
            data-slot="profile-provider-btn"
            data-active={authMethod === "api_key" || undefined}
            onClick={() => setAuthMethod("api_key")}
          >
            <Key className="h-3.5 w-3.5" />
            API Key
          </button>
        </div>
      </div>

      {authMethod === "api_key" ? (
        <div data-slot="profile-field">
          <label data-slot="profile-label" htmlFor="anthropic-key">
            API Key <span className="text-muted-foreground">(optional, uses existing if empty)</span>
          </label>
          <input
            id="anthropic-key"
            data-slot="profile-input"
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="sk-ant-..."
          />
        </div>
      ) : (
        <OAuthFlow
          oauthData={oauthData}
          oauthCode={oauthCode}
          setOauthCode={setOauthCode}
          oauthVerified={oauthVerified}
          loading={loading}
          onStart={onStartOAuth}
          onVerify={onVerifyOAuth}
        />
      )}
    </>
  );
}

// ── OAuth Flow ───────────────────────────────────────────────────────────────

interface OAuthFlowProps {
  oauthData: OAuthAuthorizeResponse | null;
  oauthCode: string;
  setOauthCode: (v: string) => void;
  oauthVerified: boolean;
  loading: boolean;
  onStart: () => void;
  onVerify: () => void;
}

function OAuthFlow({
  oauthData, oauthCode, setOauthCode, oauthVerified,
  loading, onStart, onVerify,
}: OAuthFlowProps) {
  if (oauthVerified) {
    return (
      <div data-slot="profile-oauth-status" data-verified>
        <span className="text-xs text-green-500">Authenticated — creating profile...</span>
      </div>
    );
  }

  if (!oauthData) {
    return (
      <div data-slot="profile-field">
        <button
          type="button"
          data-slot="profile-btn-primary"
          data-full-width
          onClick={onStart}
          disabled={loading}
        >
          <ExternalLink className="h-3.5 w-3.5" />
          {loading ? "Preparing..." : "Sign in with Claude"}
        </button>
      </div>
    );
  }

  return (
    <div data-slot="profile-field">
      <span data-slot="profile-label">
        Paste the authorization code from the browser:
      </span>
      <input
        data-slot="profile-input"
        value={oauthCode}
        onChange={(e) => setOauthCode(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && onVerify()}
        placeholder="Paste authorization code..."
        autoFocus
      />
      <div data-slot="profile-oauth-actions">
        <button
          type="button"
          data-slot="profile-btn-secondary"
          onClick={() => window.open(oauthData.authorize_url, "_blank")}
        >
          <ExternalLink className="h-3 w-3" />
          Re-open
        </button>
        <button
          type="button"
          data-slot="profile-btn-primary"
          onClick={onVerify}
          disabled={!oauthCode.trim() || loading}
        >
          {loading ? "Verifying..." : "Verify"}
        </button>
      </div>
    </div>
  );
}
