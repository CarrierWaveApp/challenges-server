import Foundation

// MARK: - Health

struct HealthResponse: Decodable {
    let status: String
    let version: String
    let rbn: RbnHealth?
}

struct RbnHealth: Decodable {
    let connected: Bool
    let spotsInStore: Int
    let oldestSpot: Date?
    let spotsPerMinute: Double
}

// MARK: - Challenges

struct ChallengesListResponse: Decodable {
    let challenges: [ChallengeListItem]
    let total: Int
    let limit: Int
    let offset: Int
}

struct ChallengeListItem: Decodable, Identifiable {
    let id: String
    let name: String
    let description: String?
    let category: String
    let type: String
    let participantCount: Int
    let isActive: Bool
}

// MARK: - Spots

struct SpotsListResponse: Decodable {
    let spots: [SpotItem]
    let pagination: SpotsPagination
}

struct SpotItem: Decodable, Identifiable {
    let id: String
    let callsign: String
    let programSlug: String?
    let source: String
    let frequencyKhz: Double?
    let mode: String?
    let reference: String?
    let referenceName: String?
    let spottedAt: Date?
}

struct SpotsPagination: Decodable {
    let hasMore: Bool
    let nextCursor: String?
}

// MARK: - POTA Sync Status

struct PotaSyncStatusResponse: Decodable {
    let totalParks: Int
    let parksFetched: Int
    let parksPending: Int
    let completionPercentage: Double
    let oldestFetch: Date?
    let newestFetch: Date?
    let warning: String?
}

// MARK: - Park Boundaries Status

struct ParkBoundariesStatusResponse: Decodable {
    let totalParks: Int
    let totalCached: Int
    let unfetched: Int
    let completionPercentage: Int
    let byCountry: BoundaryCountryStats
    let exactMatches: Int
    let spatialMatches: Int
    let manualMatches: Int
    let noMatches: Int?
    let oldestFetch: String?
    let newestFetch: String?
}

struct BoundaryCountryStats: Decodable {
    let us: BoundaryCountryStat
    let uk: BoundaryCountryStat
    let it: BoundaryCountryStat
    let pl: BoundaryCountryStat
}

struct BoundaryCountryStat: Decodable {
    let totalParks: Int
}

// MARK: - Historic Trails Status

struct TrailStatusResponse: Decodable {
    let totalCatalog: Int
    let totalCached: Int
    let unfetched: Int
    let completionPercentage: Int
    let exactMatches: Int
    let spatialMatches: Int
    let manualMatches: Int
    let oldestFetch: String?
    let newestFetch: String?
}

// MARK: - Programs

struct ProgramListResponse: Decodable {
    let programs: [ProgramResponse]
    let version: Int64
}

struct ProgramResponse: Decodable, Identifiable {
    var id: String { slug }
    let slug: String
    let name: String
    let shortName: String
    let icon: String
    let iconUrl: String?
    let website: String?
    let serverBaseUrl: String?
    let referenceLabel: String
    let referenceFormat: String?
    let referenceExample: String?
    let multiRefAllowed: Bool
    let activationThreshold: Int?
    let supportsRove: Bool
    let capabilities: [String]
    let isActive: Bool
}

// MARK: - Program Requests

struct CreateProgramRequest: Encodable {
    let slug: String
    let name: String
    let shortName: String
    let icon: String
    var website: String?
    let referenceLabel: String
    var referenceFormat: String?
    var referenceExample: String?
    var multiRefAllowed: Bool = false
    var activationThreshold: Int?
    var supportsRove: Bool = false
    var capabilities: [String] = []
    var sortOrder: Int = 0
}

struct UpdateProgramRequest: Encodable {
    var name: String?
    var shortName: String?
    var icon: String?
    var website: String?
    var referenceLabel: String?
    var referenceFormat: String?
    var referenceExample: String?
    var multiRefAllowed: Bool?
    var activationThreshold: Int?
    var supportsRove: Bool?
    var capabilities: [String]?
    var sortOrder: Int?
    var isActive: Bool?
}

// MARK: - Clubs (Admin)

struct ClubAdminResponse: Decodable, Identifiable {
    let id: String
    let name: String
    let callsign: String?
    let description: String?
    let notesUrl: String?
    let notesTitle: String?
    let memberCount: Int
    let createdAt: Date?
    let updatedAt: Date?
}

struct ClubMemberAdminResponse: Decodable, Identifiable {
    var id: String { callsign }
    let callsign: String
    let role: String
    let joinedAt: Date
    let lastSeenAt: Date?
    let lastGrid: String?
    let isCarrierWaveUser: Bool
}

// MARK: - Club Requests

struct CreateClubRequest: Encodable {
    let name: String
    var callsign: String?
    var description: String?
}

struct UpdateClubRequest: Encodable {
    var name: String?
    var callsign: Nullable<String>?
    var description: Nullable<String>?
    var notesUrl: Nullable<String>?
    var notesTitle: Nullable<String>?
}

/// Wrapper that encodes as `null` instead of being skipped by JSONEncoder.
/// - `Nullable<String>?.none` → key omitted (don't update)
/// - `Nullable<String>?.some(.null)` → `"key": null` (clear to NULL)
/// - `Nullable<String>?.some(.value("x"))` → `"key": "x"` (set value)
enum Nullable<T: Encodable>: Encodable {
    case null
    case value(T)

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null:
            try container.encodeNil()
        case .value(let wrapped):
            try container.encode(wrapped)
        }
    }
}

struct ImportNotesResponse: Decodable {
    let imported: Int
    let skipped: Int
    let callsigns: [String]
}

// MARK: - Admin Stats

struct AdminStatsResponse: Decodable {
    let totalUsers: Int
    let usersLast7Days: Int
    let usersLast30Days: Int
}

struct UserCountByHour: Decodable, Identifiable {
    var id: Date { hour }
    let hour: Date
    let count: Int
}

struct EmptyBody: Encodable {}

// MARK: - Events (Admin)

struct EventsListResponse: Decodable {
    let events: [EventListItem]
    let total: Int
    let limit: Int
    let offset: Int
}

struct EventListItem: Decodable, Identifiable {
    let id: String
    let name: String
    let eventType: String
    let startDate: Date
    let endDate: Date?
    let timezone: String
    let venueName: String?
    let city: String
    let state: String?
    let country: String
    let latitude: Double
    let longitude: Double
    let cost: String?
    let submittedBy: String
    let status: String
    let createdAt: Date
    let distanceMeters: Double?
}

struct EventDetailResponse: Decodable, Identifiable {
    let id: String
    let name: String
    let description: String?
    let eventType: String
    let startDate: Date
    let endDate: Date?
    let timezone: String
    let venueName: String?
    let address: String
    let city: String
    let state: String?
    let country: String
    let latitude: Double
    let longitude: Double
    let cost: String?
    let url: String?
    let submittedBy: String
    let status: String
    let reviewedBy: String?
    let reviewedAt: Date?
    let rejectionReason: String?
    let createdAt: Date
    let updatedAt: Date
}

struct ReviewEventRequest: Encodable {
    let action: String
    var reason: String?
}

struct UpdateEventRequest: Encodable {
    var name: String?
    var description: String?
    var eventType: String?
    var timezone: String?
    var venueName: String?
    var address: String?
    var city: String?
    var state: String?
    var country: String?
    var latitude: Double?
    var longitude: Double?
    var cost: String?
    var url: String?
}

struct SubmitterStatsResponse: Decodable {
    let totalSubmitted: Int
    let totalApproved: Int
    let totalRejected: Int
    let totalPending: Int
}

struct AddMembersRequest: Encodable {
    let members: [AddMemberEntry]
}

struct AddMemberEntry: Encodable {
    let callsign: String
    let role: String
}

// MARK: - Upload Error Telemetry (Admin)

struct TelemetrySummaryResponse: Decodable {
    let totalErrors: Int
    let totalAffectedQsos: Int
    let uniqueCallsigns: Int
    let byService: [ServiceErrorCount]
    let byCategory: [CategoryErrorCount]
    let dailyTrend: [DailyErrorCount]
    let recentErrors: [RecentError]
}

struct ServiceErrorCount: Decodable, Identifiable {
    var id: String { service }
    let service: String
    let errorCount: Int
    let affectedQsos: Int
}

struct CategoryErrorCount: Decodable, Identifiable {
    var id: String { category }
    let category: String
    let errorCount: Int
    let affectedQsos: Int
}

struct DailyErrorCount: Decodable, Identifiable {
    var id: String { date }
    let date: String
    let errorCount: Int
    let affectedQsos: Int
}

struct RecentError: Decodable, Identifiable {
    var id: String { "\(callsign)-\(createdAt)-\(messageHash)" }
    let service: String
    let category: String
    let messageHash: String
    let affectedCount: Int
    let isTransient: Bool
    let appVersion: String
    let osVersion: String
    let callsign: String
    let createdAt: Date
}
