-- Performance reports: track main thread hangs and other pathological performance issues
CREATE TABLE performance_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    callsign TEXT NOT NULL,
    -- What kind of performance issue
    category TEXT NOT NULL CHECK (category IN ('hang', 'slow_launch', 'memory_warning', 'crash_diagnostic', 'other')),
    -- Duration in seconds (e.g., hang duration, launch time)
    duration_seconds DOUBLE PRECISION,
    -- Freeform context about what was happening (e.g., "MapView loading", "ADIF import")
    context TEXT,
    -- Severity: info, warning, critical
    severity TEXT NOT NULL DEFAULT 'warning' CHECK (severity IN ('info', 'warning', 'critical')),
    -- Device and app metadata
    app_version TEXT,
    build_number TEXT,
    device_model TEXT,
    os_version TEXT,
    -- Optional stack trace or diagnostic payload
    diagnostic_payload JSONB,
    -- When the issue occurred on the device (may differ from created_at)
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_performance_reports_callsign ON performance_reports (callsign);
CREATE INDEX idx_performance_reports_category ON performance_reports (category, created_at DESC);
CREATE INDEX idx_performance_reports_created_at ON performance_reports (created_at DESC);
