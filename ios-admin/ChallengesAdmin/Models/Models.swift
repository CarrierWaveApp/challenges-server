import Foundation

// MARK: - Health

struct HealthResponse: Decodable {
    let status: String
    let version: String
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
    let icon: String?
    let website: String?
    let referenceLabel: String?
    let referenceFormat: String?
    let referenceExample: String?
    let multiRefAllowed: Bool?
    let activationThreshold: Int?
    let supportsRove: Bool?
    let capabilities: [String]?
    let isActive: Bool?
    let sortOrder: Int?
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
    var id: String { "\(clubId)-\(callsign)" }
    let clubId: String
    let callsign: String
    let role: String
    let joinedAt: Date?
}

// MARK: - Club Requests

struct CreateClubRequest: Encodable {
    let name: String
    var callsign: String?
    var description: String?
}

struct UpdateClubRequest: Encodable {
    var name: String?
    var callsign: String?
    var description: String?
    var notesUrl: String?
    var notesTitle: String?
}

struct AddMembersRequest: Encodable {
    let members: [AddMemberEntry]
}

struct AddMemberEntry: Encodable {
    let callsign: String
    let role: String
}
