import { CheckCircle, ExternalLink, Key, Loader2 } from "lucide-react";
import { useState } from "react";
import beeIcon from "@/assets/bee-icon.png";
import { useCompleteOAuth, useSetupApiKey, useStartOAuth } from "@/domains/settings/mutations";
import type { OAuthAuthorizeResponse } from "@/domains/settings/types";
import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/shared/ui/card";

// ── Component ────────────────────────────────────────────────────────────────

export function AuthSetup() {
  const setupApiKeyMutation = useSetupApiKey();
  const startOAuthMutation = useStartOAuth();
  const completeOAuthMutation = useCompleteOAuth();

  const [tab, setTab] = useState<"api_key" | "oauth">("api_key");
  const [apiKey, setApiKey] = useState("");
  const [oauthData, setOauthData] = useState<OAuthAuthorizeResponse | null>(null);
  const [oauthCode, setOauthCode] = useState("");
  const [success, setSuccess] = useState(false);

  const loading =
    setupApiKeyMutation.isPending ||
    startOAuthMutation.isPending ||
    completeOAuthMutation.isPending;
  const error =
    setupApiKeyMutation.error?.message ??
    startOAuthMutation.error?.message ??
    completeOAuthMutation.error?.message ??
    null;

  const handleApiKeySubmit = () => {
    if (!apiKey.trim() || loading) return;
    setupApiKeyMutation.mutate(apiKey.trim(), {
      onSuccess: () => setSuccess(true),
    });
  };

  const handleStartOAuth = () => {
    startOAuthMutation.mutate(undefined, {
      onSuccess: (data) => {
        setOauthData(data);
        window.open(data.authorize_url, "_blank");
      },
    });
  };

  const handleOAuthCallback = () => {
    if (!oauthCode.trim() || loading) return;
    completeOAuthMutation.mutate(oauthCode.trim(), {
      onSuccess: () => setSuccess(true),
    });
  };

  if (success) {
    return (
      <div
        data-component="auth-setup"
        className="flex-1 flex items-center justify-center bg-background"
      >
        <div className="flex flex-col items-center gap-4">
          <CheckCircle className="h-12 w-12 text-success" />
          <p className="text-lg font-medium text-foreground">Connected</p>
          <p className="text-sm text-muted-foreground">Starting chat...</p>
        </div>
      </div>
    );
  }

  return (
    <div
      data-component="auth-setup"
      className="flex-1 flex items-center justify-center bg-background p-4"
    >
      <div className="w-full max-w-[600px]">
        {/* Header */}
        <div className="flex flex-col items-center gap-3 mb-8">
          <img src={beeIcon} alt="Hive" className="w-16 h-16" />
          <h1 className="text-2xl font-semibold text-foreground">Connect to Claude</h1>
          <p className="text-sm text-muted-foreground text-center">
            Choose how to authenticate with the Anthropic API
          </p>
        </div>

        {/* Error */}
        {error && (
          <div className="mb-4 rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        )}

        {/* Tab buttons */}
        <div className="flex gap-2 mb-4">
          <button
            type="button"
            onClick={() => setTab("api_key")}
            className={cn(
              "flex-1 rounded-lg px-4 py-2.5 text-sm font-medium transition-colors",
              tab === "api_key"
                ? "bg-accent text-accent-foreground"
                : "bg-muted text-muted-foreground hover:text-foreground",
            )}
          >
            <Key className="inline-block h-4 w-4 mr-2 -mt-0.5" />
            API Key
          </button>
          <button
            type="button"
            onClick={() => setTab("oauth")}
            className={cn(
              "flex-1 rounded-lg px-4 py-2.5 text-sm font-medium transition-colors",
              tab === "oauth"
                ? "bg-accent text-accent-foreground"
                : "bg-muted text-muted-foreground hover:text-foreground",
            )}
          >
            <ExternalLink className="inline-block h-4 w-4 mr-2 -mt-0.5" />
            Claude Pro/Max
          </button>
        </div>

        {/* API Key tab */}
        {tab === "api_key" && (
          <Card>
            <CardHeader>
              <CardTitle>Anthropic API Key</CardTitle>
              <CardDescription>
                Enter your API key from{" "}
                <a
                  href="https://console.anthropic.com/settings/keys"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-accent hover:underline"
                >
                  console.anthropic.com
                </a>
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex flex-col gap-3">
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleApiKeySubmit()}
                  placeholder="sk-ant-..."
                  className={cn(
                    "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground",
                    "placeholder:text-muted-foreground",
                    "focus:outline-none focus:ring-1 focus:ring-ring",
                  )}
                />
                <Button
                  onClick={handleApiKeySubmit}
                  disabled={!apiKey.trim() || loading}
                  className="w-full"
                >
                  {loading ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Validating...
                    </>
                  ) : (
                    "Connect"
                  )}
                </Button>
              </div>
            </CardContent>
          </Card>
        )}

        {/* OAuth tab */}
        {tab === "oauth" && (
          <Card>
            <CardHeader>
              <CardTitle>Claude Pro / Max</CardTitle>
              <CardDescription>Sign in with your Claude subscription account</CardDescription>
            </CardHeader>
            <CardContent>
              {!oauthData ? (
                <Button onClick={handleStartOAuth} disabled={loading} className="w-full">
                  {loading ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      Preparing...
                    </>
                  ) : (
                    <>
                      <ExternalLink className="h-4 w-4" />
                      Sign in with Claude
                    </>
                  )}
                </Button>
              ) : (
                <div className="flex flex-col gap-3">
                  <p className="text-sm text-muted-foreground">
                    A browser window should have opened. After authorizing, paste the code below:
                  </p>
                  <input
                    type="text"
                    value={oauthCode}
                    onChange={(e) => setOauthCode(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleOAuthCallback()}
                    placeholder="Paste authorization code..."
                    className={cn(
                      "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground font-mono",
                      "placeholder:text-muted-foreground",
                      "focus:outline-none focus:ring-1 focus:ring-ring",
                    )}
                  />
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      onClick={() => window.open(oauthData.authorize_url, "_blank")}
                      className="flex-1"
                    >
                      <ExternalLink className="h-4 w-4" />
                      Re-open
                    </Button>
                    <Button
                      onClick={handleOAuthCallback}
                      disabled={!oauthCode.trim() || loading}
                      className="flex-1"
                    >
                      {loading ? (
                        <>
                          <Loader2 className="h-4 w-4 animate-spin" />
                          Verifying...
                        </>
                      ) : (
                        "Verify"
                      )}
                    </Button>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
