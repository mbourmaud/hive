import * as Popover from "@radix-ui/react-popover";
import { Check, ChevronRight, Cloud, Server, User } from "lucide-react";
import { useState } from "react";
import { useActivateProfile } from "../profile-mutations";
import { useActiveProfileQuery, useProfilesQuery } from "../profile-queries";
import type { ProfileInfo, ProviderType } from "../types";
import "./profile-switcher.css";

export function ProfileSwitcher() {
  const [open, setOpen] = useState(false);
  const { data: profiles } = useProfilesQuery();
  const { data: activeProfile } = useActiveProfileQuery();
  const activateMutation = useActivateProfile();

  const activeName = activeProfile?.name ?? "default";
  const activeProvider = activeProfile?.provider ?? "anthropic";
  const initial = activeName.charAt(0).toUpperCase();

  function handleActivate(name: string) {
    if (name === activeName) return;
    activateMutation.mutate(name, { onSuccess: () => setOpen(false) });
  }

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger asChild>
        <button
          type="button"
          data-slot="profile-switcher-trigger"
          title={`Profile: ${activeName} (${activeProvider})`}
        >
          <span data-slot="profile-switcher-initial">{initial}</span>
          <ProviderIcon provider={activeProvider} size={10} />
        </button>
      </Popover.Trigger>

      <Popover.Portal>
        <Popover.Content
          data-component="profile-switcher"
          side="right"
          sideOffset={8}
          align="end"
          collisionPadding={12}
        >
          <div data-slot="profile-switcher-header">
            <User className="h-3.5 w-3.5" />
            <span>Profiles</span>
          </div>

          <div data-slot="profile-switcher-list">
            {profiles?.map((profile) => (
              <ProfileRow
                key={profile.name}
                profile={profile}
                isActive={profile.name === activeName}
                isPending={activateMutation.isPending}
                onActivate={handleActivate}
              />
            ))}

            {(!profiles || profiles.length === 0) && (
              <div data-slot="profile-switcher-empty">No profiles configured</div>
            )}
          </div>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

// ── Profile Row ─────────────────────────────────────────────────────────────

function ProfileRow({
  profile,
  isActive,
  isPending,
  onActivate,
}: {
  profile: ProfileInfo;
  isActive: boolean;
  isPending: boolean;
  onActivate: (name: string) => void;
}) {
  return (
    <button
      type="button"
      data-slot="profile-switcher-item"
      data-active={isActive || undefined}
      disabled={isPending}
      onClick={() => onActivate(profile.name)}
    >
      <div data-slot="profile-switcher-item-info">
        <span data-slot="profile-switcher-item-name">{profile.name}</span>
        <ProviderBadge provider={profile.provider} />
        {!profile.has_credentials && (
          <span data-slot="profile-switcher-no-creds">No credentials</span>
        )}
      </div>
      {isActive ? (
        <Check className="h-3.5 w-3.5" data-slot="profile-switcher-check" />
      ) : (
        <ChevronRight className="h-3 w-3 opacity-0 group-hover:opacity-100" />
      )}
    </button>
  );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function ProviderIcon({ provider, size = 14 }: { provider: ProviderType; size?: number }) {
  if (provider === "bedrock") {
    return <Server style={{ width: size, height: size }} />;
  }
  return <Cloud style={{ width: size, height: size }} />;
}

function ProviderBadge({ provider }: { provider: ProviderType }) {
  return (
    <span data-slot="profile-switcher-provider" data-provider={provider}>
      {provider === "bedrock" ? "Bedrock" : "Anthropic"}
    </span>
  );
}
