-- Membership monitors: periodic URL-based membership sync for clubs
CREATE TABLE IF NOT EXISTS club_membership_monitors (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    club_id     UUID NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    url         TEXT NOT NULL,
    label       TEXT,
    format      TEXT NOT NULL DEFAULT 'callsign_notes',
    interval_hours INTEGER NOT NULL DEFAULT 24,
    last_checked_at TIMESTAMPTZ,
    last_status TEXT,
    last_member_count INTEGER,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    remove_stale BOOLEAN NOT NULL DEFAULT false,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(club_id, url)
);

CREATE INDEX idx_membership_monitors_club_id ON club_membership_monitors(club_id);
CREATE INDEX idx_membership_monitors_due ON club_membership_monitors(enabled, last_checked_at);
