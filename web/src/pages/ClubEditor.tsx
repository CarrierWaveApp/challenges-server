import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  listClubs,
  createClub,
  updateClub,
  listClubMembers,
  addClubMembers,
  removeClubMember,
  updateClubMemberRole,
} from '../api/client';
import type { Club, ClubMember } from '../types/club';

export default function ClubEditor() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const isEditing = !!id;

  const [name, setName] = useState('');
  const [callsign, setCallsign] = useState('');
  const [description, setDescription] = useState('');

  const [members, setMembers] = useState<ClubMember[]>([]);
  const [newCallsign, setNewCallsign] = useState('');
  const [newRole, setNewRole] = useState('member');

  const [loading, setLoading] = useState(!!id);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (id) {
      loadClub(id);
    }
  }, [id]);

  const loadClub = async (clubId: string) => {
    try {
      setLoading(true);
      const clubs = await listClubs();
      const club = clubs.find((c: Club) => c.id === clubId);
      if (!club) {
        setError('Club not found');
        return;
      }
      setName(club.name);
      setCallsign(club.callsign || '');
      setDescription(club.description || '');

      const memberList = await listClubMembers(clubId);
      setMembers(memberList);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load club');
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      setSaving(true);
      setError('');

      if (isEditing) {
        await updateClub(id!, {
          name,
          callsign: callsign || undefined,
          description: description || undefined,
        });
      } else {
        await createClub({
          name,
          callsign: callsign || undefined,
          description: description || undefined,
        });
      }

      navigate('/clubs');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save club');
    } finally {
      setSaving(false);
    }
  };

  const handleAddMember = async () => {
    if (!newCallsign.trim() || !id) return;
    try {
      setError('');
      await addClubMembers(id, [{ callsign: newCallsign.trim().toUpperCase(), role: newRole }]);
      setNewCallsign('');
      setNewRole('member');
      const memberList = await listClubMembers(id);
      setMembers(memberList);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add member');
    }
  };

  const handleRemoveMember = async (memberCallsign: string) => {
    if (!id || !confirm(`Remove ${memberCallsign} from the club?`)) return;
    try {
      setError('');
      await removeClubMember(id, memberCallsign);
      setMembers(members.filter((m) => m.callsign !== memberCallsign));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to remove member');
    }
  };

  const handleRoleChange = async (memberCallsign: string, role: string) => {
    if (!id) return;
    try {
      setError('');
      await updateClubMemberRole(id, memberCallsign, role);
      setMembers(
        members.map((m) =>
          m.callsign === memberCallsign ? { ...m, role } : m,
        ),
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update role');
    }
  };

  if (loading) {
    return (
      <div className="flex justify-center items-center h-64">
        <div className="text-gray-500">Loading...</div>
      </div>
    );
  }

  return (
    <div>
      <div className="sm:flex sm:items-center sm:justify-between">
        <h1 className="text-2xl font-semibold text-gray-900">
          {isEditing ? 'Edit Club' : 'New Club'}
        </h1>
        <button
          type="button"
          onClick={() => navigate('/clubs')}
          className="mt-4 sm:mt-0 inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50"
        >
          Cancel
        </button>
      </div>

      {error && (
        <div className="mt-4 rounded-md bg-red-50 p-4">
          <p className="text-sm text-red-800">{error}</p>
        </div>
      )}

      <form onSubmit={handleSave} className="mt-6 space-y-6">
        <div>
          <label className="block text-sm font-medium text-gray-700">Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700">Callsign</label>
          <input
            type="text"
            value={callsign}
            onChange={(e) => setCallsign(e.target.value.toUpperCase())}
            className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700">Description</label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            rows={3}
            className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
          />
        </div>

        <div className="flex justify-end">
          <button
            type="submit"
            disabled={saving}
            className="inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
          >
            {saving ? 'Saving...' : isEditing ? 'Update Club' : 'Create Club'}
          </button>
        </div>
      </form>

      {/* Members section - only for existing clubs */}
      {isEditing && (
        <div className="mt-10">
          <h2 className="text-lg font-medium text-gray-900 mb-4">
            Members ({members.length})
          </h2>

          <div className="flex gap-2 mb-4">
            <input
              type="text"
              placeholder="Callsign"
              value={newCallsign}
              onChange={(e) => setNewCallsign(e.target.value.toUpperCase())}
              className="flex-1 rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
            />
            <select
              value={newRole}
              onChange={(e) => setNewRole(e.target.value)}
              className="rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
            >
              <option value="member">Member</option>
              <option value="admin">Admin</option>
            </select>
            <button
              type="button"
              onClick={handleAddMember}
              disabled={!newCallsign.trim()}
              className="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md shadow-sm text-white bg-blue-600 hover:bg-blue-700 disabled:opacity-50"
            >
              Add
            </button>
          </div>

          {members.length > 0 && (
            <div className="overflow-hidden shadow ring-1 ring-black ring-opacity-5 md:rounded-lg">
              <table className="min-w-full divide-y divide-gray-300">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="py-3.5 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 sm:pl-6">
                      Callsign
                    </th>
                    <th className="px-3 py-3.5 text-left text-sm font-semibold text-gray-900">
                      Role
                    </th>
                    <th className="px-3 py-3.5 text-left text-sm font-semibold text-gray-900">
                      CW User
                    </th>
                    <th className="relative py-3.5 pl-3 pr-4 sm:pr-6">
                      <span className="sr-only">Actions</span>
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200 bg-white">
                  {members.map((member) => (
                    <tr key={member.callsign}>
                      <td className="whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-6">
                        {member.callsign}
                      </td>
                      <td className="whitespace-nowrap px-3 py-4 text-sm">
                        <select
                          value={member.role}
                          onChange={(e) => handleRoleChange(member.callsign, e.target.value)}
                          className="rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 text-sm"
                        >
                          <option value="member">member</option>
                          <option value="admin">admin</option>
                        </select>
                      </td>
                      <td className="whitespace-nowrap px-3 py-4 text-sm text-gray-500">
                        {member.isCarrierWaveUser ? (
                          <span className="inline-flex rounded-full bg-green-100 px-2 text-xs font-semibold leading-5 text-green-800">
                            Yes
                          </span>
                        ) : (
                          <span className="text-gray-400">No</span>
                        )}
                      </td>
                      <td className="relative whitespace-nowrap py-4 pl-3 pr-4 text-right text-sm font-medium sm:pr-6">
                        <button
                          onClick={() => handleRemoveMember(member.callsign)}
                          className="text-red-600 hover:text-red-900"
                        >
                          Remove
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
