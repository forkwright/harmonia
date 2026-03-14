import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useLibraryStore } from "../../library/store";
import { api } from "../../../api/client";
import type { QualityProfile, QualityProfileUpdate } from "../../../types/management";

function ProfileCard({
  profile,
  onSave,
  saving,
}: {
  profile: QualityProfile;
  onSave: (id: string, update: QualityProfileUpdate) => void;
  saving: boolean;
}) {
  const [cutoff, setCutoff] = useState(profile.cutoffQualityScore);
  const [upgradeAllowed, setUpgradeAllowed] = useState(profile.upgradeAllowed);

  function handleSave(e: React.FormEvent) {
    e.preventDefault();
    onSave(profile.id, { cutoffQualityScore: cutoff, upgradeAllowed });
  }

  return (
    <form
      onSubmit={handleSave}
      className="p-4 rounded border border-gray-700 bg-gray-800/50 space-y-3"
    >
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm font-medium text-gray-100">{profile.name}</p>
          <p className="text-xs text-gray-500 capitalize">{profile.mediaType}</p>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="block text-xs text-gray-400 mb-1">Quality Cutoff</label>
          <input
            type="number"
            min={0}
            max={100}
            value={cutoff}
            onChange={(e) => setCutoff(Number(e.target.value))}
            className="w-full px-2 py-1 text-sm rounded bg-gray-700 border border-gray-600 text-gray-100 focus:outline-none"
          />
        </div>
        <div className="flex items-end gap-2 pb-1">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={upgradeAllowed}
              onChange={(e) => setUpgradeAllowed(e.target.checked)}
              className="rounded border-gray-600 bg-gray-700"
            />
            <span className="text-xs text-gray-300">Allow Upgrades</span>
          </label>
        </div>
      </div>
      {profile.preferredCodecs.length > 0 && (
        <p className="text-xs text-gray-500">
          Preferred: {profile.preferredCodecs.join(", ")}
        </p>
      )}
      <button
        type="submit"
        disabled={saving}
        className="px-3 py-1.5 text-sm rounded bg-blue-600 hover:bg-blue-500 disabled:opacity-50 text-white transition-colors"
      >
        {saving ? "Saving…" : "Save"}
      </button>
    </form>
  );
}

export default function QualityProfilesPage() {
  const token = useLibraryStore((s) => s.token);
  const queryClient = useQueryClient();
  const [savingId, setSavingId] = useState<string | null>(null);

  const profiles = useQuery({
    queryKey: ["manage-quality-profiles", token],
    queryFn: () => api.getQualityProfiles(token),
    enabled: token.length > 0,
  });

  const updateProfile = useMutation({
    mutationFn: ({ id, update }: { id: string; update: QualityProfileUpdate }) =>
      api.updateQualityProfile(id, update, token),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["manage-quality-profiles"] });
      setSavingId(null);
    },
    onError: () => setSavingId(null),
  });

  function handleSave(id: string, update: QualityProfileUpdate) {
    setSavingId(id);
    updateProfile.mutate({ id, update });
  }

  const profileList: QualityProfile[] = profiles.data ?? [];

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <h1 className="text-xl font-semibold text-gray-100">Quality Profiles</h1>
      {profiles.isLoading && <p className="text-sm text-gray-500">Loading…</p>}
      {profiles.isError && (
        <p className="text-sm text-red-400">Failed to load quality profiles.</p>
      )}
      {!profiles.isLoading && profileList.length === 0 && (
        <p className="text-sm text-gray-500">No quality profiles configured.</p>
      )}
      <div className="grid grid-cols-2 gap-4">
        {profileList.map((profile) => (
          <ProfileCard
            key={profile.id}
            profile={profile}
            onSave={handleSave}
            saving={savingId === profile.id && updateProfile.isPending}
          />
        ))}
      </div>
    </div>
  );
}
