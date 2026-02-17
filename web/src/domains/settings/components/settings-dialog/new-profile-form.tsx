import { Cloud, Server } from "lucide-react";
import { useCallback, useState } from "react";
import { useCreateProfile } from "@/domains/settings/profile-mutations";
import { useCompleteOAuth, useStartOAuth } from "@/domains/settings/mutations";
import { useAwsProfiles } from "@/domains/settings/use-aws-profiles";
import type { CreateProfileParams, OAuthAuthorizeResponse, ProviderType } from "@/domains/settings/types";
import { AnthropicFields, BedrockFields, type AnthropicAuthMethod, type BedrockAuthMethod } from "./profile-form-fields";

// ── Component ────────────────────────────────────────────────────────────────

export function NewProfileForm({ onClose }: { onClose: () => void }) {
  const createProfile = useCreateProfile();
  const startOAuth = useStartOAuth();
  const completeOAuth = useCompleteOAuth();

  const [name, setName] = useState("");
  const [provider, setProvider] = useState<ProviderType>("anthropic");
  const [authMethod, setAuthMethod] = useState<AnthropicAuthMethod>("oauth");
  const [bedrockAuth, setBedrockAuth] = useState<BedrockAuthMethod>("aws_profile");

  // Bedrock fields
  const [region, setRegion] = useState("us-east-1");
  const [accessKeyId, setAccessKeyId] = useState("");
  const [secretAccessKey, setSecretAccessKey] = useState("");
  const [sessionToken, setSessionToken] = useState("");
  const [awsProfile, setAwsProfile] = useState("");

  // Fetch available AWS profiles when Bedrock is selected
  const { data: awsProfiles } = useAwsProfiles(provider === "bedrock");

  // Anthropic API key field
  const [apiKey, setApiKey] = useState("");

  // OAuth flow state
  const [oauthData, setOauthData] = useState<OAuthAuthorizeResponse | null>(null);
  const [oauthCode, setOauthCode] = useState("");
  const [oauthVerified, setOauthVerified] = useState(false);

  const [error, setError] = useState<string | null>(null);

  const loading =
    createProfile.isPending || startOAuth.isPending || completeOAuth.isPending;

  const handleStartOAuth = useCallback(() => {
    if (!name.trim()) {
      setError("Profile name is required");
      return;
    }
    setError(null);
    startOAuth.mutate(undefined, {
      onSuccess: (data) => {
        setOauthData(data);
        window.open(data.authorize_url, "_blank");
      },
      onError: (err) => setError(err.message),
    });
  }, [name, startOAuth]);

  const handleVerifyOAuth = useCallback(() => {
    if (!oauthCode.trim() || loading) return;
    setError(null);
    completeOAuth.mutate(oauthCode.trim(), {
      onSuccess: () => {
        setOauthVerified(true);
        createProfile.mutate(
          { name, provider: "anthropic" },
          {
            onSuccess: () => onClose(),
            onError: (err) => setError(err.message),
          },
        );
      },
      onError: (err) => setError(err.message),
    });
  }, [oauthCode, loading, completeOAuth, createProfile, name, onClose]);

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      setError(null);

      const params: CreateProfileParams = { name, provider };

      if (provider === "bedrock") {
        if (!region) {
          setError("Region is required");
          return;
        }
        params.region = region;

        if (bedrockAuth === "aws_profile") {
          if (!awsProfile) {
            setError("AWS Profile name is required");
            return;
          }
          params.aws_profile = awsProfile;
        } else {
          if (!accessKeyId || !secretAccessKey) {
            setError("Access Key ID and Secret Access Key are required");
            return;
          }
          params.access_key_id = accessKeyId;
          params.secret_access_key = secretAccessKey;
          if (sessionToken) params.session_token = sessionToken;
        }
      } else if (authMethod === "api_key" && apiKey) {
        params.api_key = apiKey;
      }

      createProfile.mutate(params, {
        onSuccess: () => onClose(),
        onError: (err) => setError(err.message),
      });
    },
    [name, provider, region, accessKeyId, secretAccessKey, sessionToken, apiKey, authMethod, bedrockAuth, awsProfile, createProfile, onClose],
  );

  const isOAuthFlow = provider === "anthropic" && authMethod === "oauth";

  return (
    <form data-slot="profile-form" onSubmit={isOAuthFlow ? (e) => e.preventDefault() : handleSubmit}>
      <div data-slot="profile-field">
        <label data-slot="profile-label" htmlFor="profile-name">Name</label>
        <input
          id="profile-name"
          data-slot="profile-input"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="my-profile"
          required
        />
      </div>

      <div data-slot="profile-field">
        <span data-slot="profile-label">Provider</span>
        <div data-slot="profile-provider-selector">
          <button
            type="button"
            data-slot="profile-provider-btn"
            data-active={provider === "anthropic" || undefined}
            onClick={() => setProvider("anthropic")}
          >
            <Cloud className="h-3.5 w-3.5" />
            Anthropic
          </button>
          <button
            type="button"
            data-slot="profile-provider-btn"
            data-active={provider === "bedrock" || undefined}
            onClick={() => setProvider("bedrock")}
          >
            <Server className="h-3.5 w-3.5" />
            Bedrock
          </button>
        </div>
      </div>

      {provider === "bedrock" ? (
        <BedrockFields
          bedrockAuth={bedrockAuth}
          setBedrockAuth={setBedrockAuth}
          region={region}
          setRegion={setRegion}
          awsProfile={awsProfile}
          setAwsProfile={setAwsProfile}
          awsProfiles={awsProfiles ?? []}
          accessKeyId={accessKeyId}
          setAccessKeyId={setAccessKeyId}
          secretAccessKey={secretAccessKey}
          setSecretAccessKey={setSecretAccessKey}
          sessionToken={sessionToken}
          setSessionToken={setSessionToken}
        />
      ) : (
        <AnthropicFields
          authMethod={authMethod}
          setAuthMethod={setAuthMethod}
          apiKey={apiKey}
          setApiKey={setApiKey}
          oauthData={oauthData}
          oauthCode={oauthCode}
          setOauthCode={setOauthCode}
          oauthVerified={oauthVerified}
          loading={loading}
          onStartOAuth={handleStartOAuth}
          onVerifyOAuth={handleVerifyOAuth}
        />
      )}

      {error ? <p data-slot="profile-error">{error}</p> : null}

      <div data-slot="profile-actions">
        <button type="button" data-slot="profile-btn-secondary" onClick={onClose}>
          Cancel
        </button>
        {!isOAuthFlow ? (
          <button type="submit" data-slot="profile-btn-primary" disabled={loading}>
            {loading ? "Creating..." : "Create Profile"}
          </button>
        ) : null}
      </div>
    </form>
  );
}
