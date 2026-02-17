import { Key, User } from "lucide-react";
import type { AwsProfileInfo } from "@/domains/settings/types";

// Re-export from extracted file
export { AnthropicFields } from "./anthropic-fields";

// ── Types ────────────────────────────────────────────────────────────────────

export type AnthropicAuthMethod = "oauth" | "api_key";
export type BedrockAuthMethod = "aws_profile" | "static_keys";

// ── Bedrock Fields ───────────────────────────────────────────────────────────

interface BedrockFieldsProps {
  bedrockAuth: BedrockAuthMethod;
  setBedrockAuth: (v: BedrockAuthMethod) => void;
  region: string;
  setRegion: (v: string) => void;
  awsProfile: string;
  setAwsProfile: (v: string) => void;
  awsProfiles: AwsProfileInfo[];
  accessKeyId: string;
  setAccessKeyId: (v: string) => void;
  secretAccessKey: string;
  setSecretAccessKey: (v: string) => void;
  sessionToken: string;
  setSessionToken: (v: string) => void;
}

export function BedrockFields({
  bedrockAuth, setBedrockAuth,
  region, setRegion,
  awsProfile, setAwsProfile,
  awsProfiles,
  accessKeyId, setAccessKeyId,
  secretAccessKey, setSecretAccessKey,
  sessionToken, setSessionToken,
}: BedrockFieldsProps) {
  function handleProfileSelect(profileName: string) {
    setAwsProfile(profileName);
    const match = awsProfiles.find((p) => p.name === profileName);
    if (match?.region) {
      setRegion(match.region);
    }
  }

  return (
    <>
      <div data-slot="profile-field">
        <span data-slot="profile-label">Auth method</span>
        <div data-slot="profile-provider-selector">
          <button
            type="button"
            data-slot="profile-provider-btn"
            data-active={bedrockAuth === "aws_profile" || undefined}
            onClick={() => setBedrockAuth("aws_profile")}
          >
            <User className="h-3.5 w-3.5" />
            AWS Profile
          </button>
          <button
            type="button"
            data-slot="profile-provider-btn"
            data-active={bedrockAuth === "static_keys" || undefined}
            onClick={() => setBedrockAuth("static_keys")}
          >
            <Key className="h-3.5 w-3.5" />
            Static Keys
          </button>
        </div>
      </div>

      <div data-slot="profile-field">
        <label data-slot="profile-label" htmlFor="bedrock-region">Region</label>
        <input
          id="bedrock-region"
          data-slot="profile-input"
          value={region}
          onChange={(e) => setRegion(e.target.value)}
          placeholder="us-east-1"
        />
      </div>

      {bedrockAuth === "aws_profile" ? (
        <AwsProfileField
          awsProfile={awsProfile}
          onSelect={handleProfileSelect}
          awsProfiles={awsProfiles}
        />
      ) : (
        <StaticKeysFields
          accessKeyId={accessKeyId}
          setAccessKeyId={setAccessKeyId}
          secretAccessKey={secretAccessKey}
          setSecretAccessKey={setSecretAccessKey}
          sessionToken={sessionToken}
          setSessionToken={setSessionToken}
        />
      )}
    </>
  );
}

// ── AWS Profile Field ───────────────────────────────────────────────────────

interface AwsProfileFieldProps {
  awsProfile: string;
  onSelect: (name: string) => void;
  awsProfiles: AwsProfileInfo[];
}

function AwsProfileField({ awsProfile, onSelect, awsProfiles }: AwsProfileFieldProps) {
  const listId = "aws-profiles-list";
  return (
    <div data-slot="profile-field">
      <label data-slot="profile-label" htmlFor="bedrock-aws-profile">
        AWS Profile
        {awsProfiles.length > 0 ? (
          <span className="text-muted-foreground"> ({awsProfiles.length} found)</span>
        ) : null}
      </label>
      <input
        id="bedrock-aws-profile"
        data-slot="profile-input"
        value={awsProfile}
        onChange={(e) => onSelect(e.target.value)}
        placeholder="default"
        list={listId}
        required
      />
      {awsProfiles.length > 0 ? (
        <datalist id={listId}>
          {awsProfiles.map((p) => (
            <option key={p.name} value={p.name}>
              {p.region ? `${p.name} (${p.region})` : p.name}
            </option>
          ))}
        </datalist>
      ) : null}
    </div>
  );
}

// ── Static Keys Fields ──────────────────────────────────────────────────────

interface StaticKeysFieldsProps {
  accessKeyId: string;
  setAccessKeyId: (v: string) => void;
  secretAccessKey: string;
  setSecretAccessKey: (v: string) => void;
  sessionToken: string;
  setSessionToken: (v: string) => void;
}

function StaticKeysFields({
  accessKeyId, setAccessKeyId,
  secretAccessKey, setSecretAccessKey,
  sessionToken, setSessionToken,
}: StaticKeysFieldsProps) {
  return (
    <>
      <div data-slot="profile-field">
        <label data-slot="profile-label" htmlFor="bedrock-access-key">Access Key ID</label>
        <input
          id="bedrock-access-key"
          data-slot="profile-input"
          value={accessKeyId}
          onChange={(e) => setAccessKeyId(e.target.value)}
          required
        />
      </div>
      <div data-slot="profile-field">
        <label data-slot="profile-label" htmlFor="bedrock-secret">Secret Access Key</label>
        <input
          id="bedrock-secret"
          data-slot="profile-input"
          type="password"
          value={secretAccessKey}
          onChange={(e) => setSecretAccessKey(e.target.value)}
          required
        />
      </div>
      <div data-slot="profile-field">
        <label data-slot="profile-label" htmlFor="bedrock-token">
          Session Token <span className="text-muted-foreground">(optional)</span>
        </label>
        <input
          id="bedrock-token"
          data-slot="profile-input"
          type="password"
          value={sessionToken}
          onChange={(e) => setSessionToken(e.target.value)}
        />
      </div>
    </>
  );
}
