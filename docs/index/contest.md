# Contest Definitions Index

Self-contained module that loads, validates, and (eventually) scores
amateur radio contest definitions in the v0.3 JSON format.

See [docs/features/contest-definitions.md](../features/contest-definitions.md)
for the format reference.

## Files

### `src/contest/mod.rs`
Module root with public re-exports and integration tests against the
example JSON files in `docs/examples/contests/`.

**Exports:**
- Re-exports all public items from `types` and `validation`

**Tests:**
- Parses and validates each example file (cwt, sst, mst, cqww-cw, sample-bundle)
- Round-trip serialize/deserialize
- Negative tests: unknown band, unknown mode, forward phase ref, unknown
  condition field, received-side serial with auto_increment
- Warning test: per_qso phase whose last rule has non-empty conditions

### `src/contest/types.rs`
Serde data types for the format.

**Exports:**
- `struct ContestDefinition` - Top-level file: `version` + `Vec<Contest>`
- `struct Contest` - Single contest definition with all sections
- `struct Sponsor` - `{ name, url }`
- `struct DataDependencies` - Map of well-known dependency keys to `DataDependency`
- `struct DataDependency` - `{ id, url, file, description, update_frequency, provides, options }`
- `enum UpdateFrequency` - static, monthly, weekly, before_each_contest
- `enum Schedule` - Tagged on `type`: RecurringWeekly, Annual, FixedDates
- `struct WeeklySession` - `{ day, start_utc, duration_minutes }`
- `enum SessionScoring` - independent, cumulative
- `struct FixedOccurrence` - `{ start, duration_hours }`
- `struct Exchange` - `{ sent: Vec<ExchangeField>, received: Vec<ExchangeField> }`
- `struct ExchangeField` - Single exchange field with type, label, autofill, etc.
- `struct DupeRules` - `{ scope, key, zero_point_qsos_count_for_mults }`
- `enum DupeScope` - session, contest
- `struct Scoring` - `{ phases: Vec<Phase> }`
- `enum Phase` - Tagged on `type`: PerQso, MultiplierCount, Aggregate
- `impl Phase::id()` - Returns the phase's id regardless of variant
- `enum MultiplierScope` - per_band, per_contest
- `struct Rule` - `{ description, conditions, value }` for per_qso rules
- `struct Condition` - `{ field, op, value, ref }` (value/ref mutually exclusive at validation time)
- `enum ConditionOp` - eq, ne, in, not_in, gt, gte, lt, lte
- `enum AggregateOp` - Tagged on `op`: Ref, Literal, Add, Multiply, Max
- `struct OperatingConstraints` - `{ off_time, band_changes }`
- `struct OffTimeRules` - `{ minimum_off_minutes, maximum_on_hours_by_category }`
- `struct BandChangeRules` - `{ applies_to, min_minutes_on_band, max_changes_per_clock_hour }`
- `struct Cabrillo` - `{ contest_name, version, qso_format }`
- `struct Category` - Entry category with id, label, band, power_classes, op_count
- `enum OpCount` - single, multi, multi_two, multi_multi
- `struct OverlayCategory` - Stacking overlay category with optional max_on_hours / max_license_years

### `src/contest/validation.rs`
Semantic validation pass that runs after structural deserialization.

**Exports:**
- `struct ValidationError` - `{ severity, contest_id, path, message }` with `Display`
- `enum Severity` - Error, Warning
- `impl ContestDefinition::validate()` - Returns `Vec<ValidationError>`
- `impl ContestDefinition::is_valid()` - True iff there are no Error-severity entries

**Validation rules:**
- Mode and band vocabulary
- Schedule sanity (HH:MM, lowercase day names, positive durations)
- Exchange field types, duplicate names, serial_number / auto_increment consistency
- Dupe rules: at least one key component, allowed key components only
- Phase id uniqueness, per_qso fallback rule warning
- Condition field vocabulary (derived fields + received.* fields from exchange)
- Condition value/ref mutual exclusion, in/not_in requires array literal
- Continent literal warning for `their_continent` / `my_continent` equality
- Multiplier source must be derived field or defined received.* field
- Aggregate phase forward-ref check (phase DAG ordering)
- Aggregate add/multiply/max requires non-empty inputs
- Data dependency satisfaction warning (their_country etc. used but no provider declared)
- Category id uniqueness across regular and overlay categories
- Single-band category band must be a known band
