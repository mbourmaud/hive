import { Check, Plus, Trash2 } from "lucide-react";
import { useCallback, useState } from "react";
import {
  useActivateProfile,
  useDeleteProfile,
} from "@/domains/settings/profile-mutations";
import { useProfilesQuery } from "@/domains/settings/profile-queries";
import { NewProfileForm } from "./new-profile-form";
import "./profiles-tab.css";

// ── Profile List ────────────────────────────────────────────────────────────

export function ProfilesTab() {
  const { data: profiles, isLoading } = useProfilesQuery();
  const activateProfile = useActivateProfile();
  const deleteProfile = useDeleteProfile();
  const [showForm, setShowForm] = useState(false);

  const handleDelete = useCallback(
    (name: string) => {
      deleteProfile.mutate(name);
    },
    [deleteProfile],
  );

  if (isLoading) {
    return (
      <div data-slot="settings-group">
        <p className="text-sm text-muted-foreground">Loading profiles...</p>
      </div>
    );
  }

  return (
    <div data-component="profiles-tab">
      <div data-slot="settings-group">
        <div data-slot="profile-header">
          <span data-slot="settings-label">Profiles</span>
          <button
            type="button"
            data-slot="profile-add-btn"
            onClick={() => setShowForm(true)}
          >
            <Plus className="h-3.5 w-3.5" />
            New
          </button>
        </div>

        {showForm ? <NewProfileForm onClose={() => setShowForm(false)} /> : null}

        <div data-slot="profile-list">
          {profiles?.map((p) => (
            <div key={p.name} data-slot="profile-item" data-active={p.is_active || undefined}>
              <div data-slot="profile-item-info">
                <div data-slot="profile-item-name">
                  {p.is_active ? <Check className="h-3.5 w-3.5 text-green-500" /> : null}
                  {p.name}
                  <span data-slot="profile-badge">{p.provider}</span>
                </div>
                {p.description ? (
                  <span data-slot="profile-item-desc">{p.description}</span>
                ) : null}
              </div>
              <div data-slot="profile-item-actions">
                {!p.is_active ? (
                  <button
                    type="button"
                    data-slot="profile-btn-secondary"
                    onClick={() => activateProfile.mutate(p.name)}
                    disabled={activateProfile.isPending}
                  >
                    Activate
                  </button>
                ) : null}
                {p.name !== "default" ? (
                  <button
                    type="button"
                    data-slot="profile-btn-danger"
                    onClick={() => handleDelete(p.name)}
                    disabled={deleteProfile.isPending}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                ) : null}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
