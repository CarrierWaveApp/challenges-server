import type { Challenge, ChallengeListItem, Badge, Invite } from '../types/challenge';
import type { Program, CreateProgramRequest, UpdateProgramRequest } from '../types/program';
import type { Club, ClubMember } from '../types/club';

const API_BASE = '/v1';
const TOKEN_KEY = 'challenges_admin_token';

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string): void {
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearToken(): void {
  localStorage.removeItem(TOKEN_KEY);
}

interface ApiError {
  error: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
  };
}

export async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: { code: 'UNKNOWN', message: response.statusText },
    }));
    throw new Error(error.error.message);
  }
  const data = await response.json();
  return data.data as T;
}

export function authHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

// Challenges
export async function listChallenges(params?: {
  category?: string;
  type?: string;
  active?: boolean;
}): Promise<{ challenges: ChallengeListItem[]; total: number }> {
  const searchParams = new URLSearchParams();
  if (params?.category) searchParams.set('category', params.category);
  if (params?.type) searchParams.set('type', params.type);
  if (params?.active !== undefined) searchParams.set('active', String(params.active));

  const url = `${API_BASE}/challenges${searchParams.toString() ? '?' + searchParams : ''}`;
  const response = await fetch(url, { headers: authHeaders() });
  return handleResponse(response);
}

export async function getChallenge(id: string): Promise<Challenge> {
  const response = await fetch(`${API_BASE}/challenges/${id}`, {
    headers: authHeaders(),
  });
  return handleResponse(response);
}

export async function createChallenge(challenge: Omit<Challenge, 'id' | 'version' | 'createdAt' | 'updatedAt'>): Promise<Challenge> {
  const response = await fetch(`${API_BASE}/admin/challenges`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify(challenge),
  });
  return handleResponse(response);
}

export async function updateChallenge(id: string, challenge: Omit<Challenge, 'id' | 'version' | 'createdAt' | 'updatedAt'>): Promise<Challenge> {
  const response = await fetch(`${API_BASE}/admin/challenges/${id}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify(challenge),
  });
  return handleResponse(response);
}

export async function deleteChallenge(id: string): Promise<void> {
  const response = await fetch(`${API_BASE}/admin/challenges/${id}`, {
    method: 'DELETE',
    headers: authHeaders(),
  });
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: { code: 'UNKNOWN', message: response.statusText },
    }));
    throw new Error(error.error.message);
  }
}

// Badges
export async function listBadges(challengeId: string): Promise<{ badges: Badge[] }> {
  const response = await fetch(`${API_BASE}/admin/challenges/${challengeId}/badges`, {
    headers: authHeaders(),
  });
  return handleResponse(response);
}

export async function uploadBadge(
  challengeId: string,
  file: File,
  name: string,
  tierId?: string
): Promise<Badge> {
  const formData = new FormData();
  formData.append('image', file);
  formData.append('name', name);
  if (tierId) formData.append('tierId', tierId);

  const token = getToken();
  const response = await fetch(`${API_BASE}/admin/challenges/${challengeId}/badges`, {
    method: 'POST',
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    body: formData,
  });
  return handleResponse(response);
}

export async function deleteBadge(badgeId: string): Promise<void> {
  const response = await fetch(`${API_BASE}/admin/badges/${badgeId}`, {
    method: 'DELETE',
    headers: authHeaders(),
  });
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: { code: 'UNKNOWN', message: response.statusText },
    }));
    throw new Error(error.error.message);
  }
}

// Invites
export async function listInvites(challengeId: string): Promise<{ invites: Invite[] }> {
  const response = await fetch(`${API_BASE}/admin/challenges/${challengeId}/invites`, {
    headers: authHeaders(),
  });
  return handleResponse(response);
}

export async function generateInvite(
  challengeId: string,
  maxUses?: number,
  expiresAt?: string
): Promise<Invite> {
  const response = await fetch(`${API_BASE}/admin/challenges/${challengeId}/invites`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify({ maxUses, expiresAt }),
  });
  return handleResponse(response);
}

export async function revokeInvite(token: string): Promise<void> {
  const response = await fetch(`${API_BASE}/admin/invites/${token}`, {
    method: 'DELETE',
    headers: authHeaders(),
  });
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: { code: 'UNKNOWN', message: response.statusText },
    }));
    throw new Error(error.error.message);
  }
}

// Programs
export async function listPrograms(): Promise<{ programs: Program[]; version: number }> {
  const response = await fetch(`${API_BASE}/admin/programs`, {
    headers: authHeaders(),
  });
  return handleResponse(response);
}

export async function getProgram(slug: string): Promise<Program> {
  const response = await fetch(`${API_BASE}/admin/programs/${slug}`, {
    headers: authHeaders(),
  });
  return handleResponse(response);
}

export async function createProgram(program: CreateProgramRequest): Promise<Program> {
  const response = await fetch(`${API_BASE}/admin/programs`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify(program),
  });
  return handleResponse(response);
}

export async function updateProgram(slug: string, program: UpdateProgramRequest): Promise<Program> {
  const response = await fetch(`${API_BASE}/admin/programs/${slug}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify(program),
  });
  return handleResponse(response);
}

export async function deleteProgram(slug: string): Promise<void> {
  const response = await fetch(`${API_BASE}/admin/programs/${slug}`, {
    method: 'DELETE',
    headers: authHeaders(),
  });
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: { code: 'UNKNOWN', message: response.statusText },
    }));
    throw new Error(error.error.message);
  }
}

// Clubs
export async function listClubs(): Promise<Club[]> {
  const response = await fetch(`${API_BASE}/admin/clubs`, {
    headers: authHeaders(),
  });
  return handleResponse(response);
}

export async function createClub(club: { name: string; callsign?: string; description?: string }): Promise<Club> {
  const response = await fetch(`${API_BASE}/admin/clubs`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify(club),
  });
  return handleResponse(response);
}

export async function updateClub(
  id: string,
  club: {
    name?: string;
    callsign?: string | null;
    description?: string | null;
    notesUrl?: string | null;
    notesTitle?: string | null;
  },
): Promise<Club> {
  const response = await fetch(`${API_BASE}/admin/clubs/${id}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify(club),
  });
  return handleResponse(response);
}

export async function importNotesMembers(
  clubId: string,
): Promise<{ imported: number; skipped: number; callsigns: string[] }> {
  const response = await fetch(`${API_BASE}/admin/clubs/${clubId}/import-notes`, {
    method: 'POST',
    headers: authHeaders(),
  });
  return handleResponse(response);
}

export async function deleteClub(id: string): Promise<void> {
  const response = await fetch(`${API_BASE}/admin/clubs/${id}`, {
    method: 'DELETE',
    headers: authHeaders(),
  });
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: { code: 'UNKNOWN', message: response.statusText },
    }));
    throw new Error(error.error.message);
  }
}

export async function listClubMembers(clubId: string): Promise<ClubMember[]> {
  const response = await fetch(`${API_BASE}/admin/clubs/${clubId}/members`, {
    headers: authHeaders(),
  });
  return handleResponse(response);
}

export async function addClubMembers(
  clubId: string,
  members: { callsign: string; role?: string }[],
): Promise<ClubMember[]> {
  const response = await fetch(`${API_BASE}/admin/clubs/${clubId}/members`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify({ members }),
  });
  return handleResponse(response);
}

export async function removeClubMember(clubId: string, callsign: string): Promise<void> {
  const response = await fetch(`${API_BASE}/admin/clubs/${clubId}/members/${encodeURIComponent(callsign)}`, {
    method: 'DELETE',
    headers: authHeaders(),
  });
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: { code: 'UNKNOWN', message: response.statusText },
    }));
    throw new Error(error.error.message);
  }
}

export async function updateClubMemberRole(
  clubId: string,
  callsign: string,
  role: string,
): Promise<ClubMember> {
  const response = await fetch(`${API_BASE}/admin/clubs/${clubId}/members/${encodeURIComponent(callsign)}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
    },
    body: JSON.stringify({ role }),
  });
  return handleResponse(response);
}
