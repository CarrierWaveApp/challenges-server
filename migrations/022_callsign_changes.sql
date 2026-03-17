-- Track callsign changes for audit and app sync
CREATE TABLE callsign_changes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    old_callsign    TEXT NOT NULL,
    new_callsign    TEXT NOT NULL,
    changed_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_callsign_changes_user_id ON callsign_changes(user_id);
CREATE INDEX idx_callsign_changes_old_callsign ON callsign_changes(old_callsign);
