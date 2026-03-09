import SwiftUI

struct DashboardView: View {
    @EnvironmentObject var config: ServerConfig
    @State private var health: HealthResponse?
    @State private var challenges: ChallengesListResponse?
    @State private var potaStatus: PotaSyncStatusResponse?
    @State private var spotsResponse: SpotsListResponse?
    @State private var clubs: [ClubAdminResponse]?
    @State private var error: String?
    @State private var isLoading = true
    @State private var lastRefresh: Date?

    private var api: APIClient { APIClient(config: config) }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    if let error {
                        ErrorBanner(message: error)
                    }

                    if isLoading && health == nil {
                        ProgressView("Connecting to server...")
                            .padding(.top, 40)
                    } else {
                        serverStatusSection
                        countsSection
                        aggregatorSummarySection
                        if let lastRefresh {
                            Text("Last refreshed \(lastRefresh.formatted(.relative(presentation: .named)))")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .padding(.bottom)
                        }
                    }
                }
                .padding()
            }
            .navigationTitle("Dashboard")
            .refreshable { await loadAll() }
            .task { await loadAll() }
        }
    }

    // MARK: - Server Status

    private var serverStatusSection: some View {
        VStack(spacing: 12) {
            HStack {
                Text("Server")
                    .font(.headline)
                Spacer()
            }

            HStack(spacing: 16) {
                StatusCard(
                    title: "Status",
                    value: health?.status.uppercased() ?? "UNKNOWN",
                    icon: health?.status == "ok" ? "checkmark.circle.fill" : "exclamationmark.triangle.fill",
                    color: health?.status == "ok" ? .green : .red
                )

                StatusCard(
                    title: "Version",
                    value: health?.version ?? "-",
                    icon: "tag",
                    color: .blue
                )
            }

            HStack {
                Image(systemName: "link")
                    .foregroundStyle(.secondary)
                Text(config.baseURL)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer()
            }
            .padding(.horizontal, 4)
        }
        .cardStyle()
    }

    // MARK: - Counts

    private var countsSection: some View {
        VStack(spacing: 12) {
            HStack {
                Text("At a Glance")
                    .font(.headline)
                Spacer()
            }

            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible()),
            ], spacing: 12) {
                CountCard(
                    title: "Challenges",
                    count: challenges?.total,
                    icon: "trophy.fill",
                    color: .orange
                )
                CountCard(
                    title: "Active",
                    count: challenges?.challenges.filter(\.isActive).count,
                    icon: "bolt.fill",
                    color: .green
                )
                CountCard(
                    title: "Clubs",
                    count: clubs?.count,
                    icon: "person.3.fill",
                    color: .purple
                )
                CountCard(
                    title: "Total Members",
                    count: clubs?.reduce(0) { $0 + $1.memberCount },
                    icon: "person.fill",
                    color: .indigo
                )
            }
        }
        .cardStyle()
    }

    // MARK: - Aggregator Summary

    private var aggregatorSummarySection: some View {
        VStack(spacing: 12) {
            HStack {
                Text("Aggregators")
                    .font(.headline)
                Spacer()
                NavigationLink {
                    AggregatorsView()
                } label: {
                    Text("Details")
                        .font(.subheadline)
                }
            }

            if let pota = potaStatus {
                AggregatorRow(
                    name: "POTA Stats",
                    progress: pota.completionPercentage / 100.0,
                    detail: "\(pota.parksFetched)/\(pota.totalParks) parks"
                )
            }
        }
        .cardStyle()
    }

    // MARK: - Data Loading

    private func loadAll() async {
        isLoading = true
        error = nil

        await withTaskGroup(of: Void.self) { group in
            group.addTask { await loadHealth() }
            group.addTask { await loadChallenges() }
            group.addTask { await loadPotaStatus() }
            group.addTask { await loadClubs() }
        }

        lastRefresh = Date()
        isLoading = false
    }

    private func loadHealth() async {
        do {
            health = try await api.getHealth()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func loadChallenges() async {
        do {
            challenges = try await api.getChallenges()
        } catch {
            // Non-critical, don't override error
        }
    }

    private func loadPotaStatus() async {
        do {
            potaStatus = try await api.getPotaSyncStatus()
        } catch {
            // Non-critical
        }
    }

    private func loadClubs() async {
        do {
            clubs = try await api.getClubs()
        } catch {
            // Non-critical
        }
    }
}

// MARK: - Subviews

struct StatusCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color

    var body: some View {
        VStack(spacing: 6) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundStyle(color)
            Text(value)
                .font(.headline)
                .fontDesign(.monospaced)
            Text(title)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding()
        .background(color.opacity(0.1))
        .clipShape(RoundedRectangle(cornerRadius: 10))
    }
}

struct CountCard: View {
    let title: String
    let count: Int?
    let icon: String
    let color: Color

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: icon)
                .font(.title3)
                .foregroundStyle(color)
                .frame(width: 28)

            VStack(alignment: .leading, spacing: 2) {
                Text(count.map { "\($0)" } ?? "-")
                    .font(.title3.bold())
                    .fontDesign(.monospaced)
                Text(title)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
        }
        .padding(12)
        .background(color.opacity(0.08))
        .clipShape(RoundedRectangle(cornerRadius: 10))
    }
}

struct AggregatorRow: View {
    let name: String
    let progress: Double
    let detail: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(name)
                    .font(.subheadline.weight(.medium))
                Spacer()
                Text(detail)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            ProgressView(value: min(progress, 1.0))
                .tint(progress >= 1.0 ? .green : .blue)
        }
    }
}

struct ErrorBanner: View {
    let message: String

    var body: some View {
        HStack {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.yellow)
            Text(message)
                .font(.subheadline)
            Spacer()
        }
        .padding()
        .background(.red.opacity(0.1))
        .clipShape(RoundedRectangle(cornerRadius: 10))
    }
}

// MARK: - Card Style

extension View {
    func cardStyle() -> some View {
        self
            .padding()
            .background(.regularMaterial)
            .clipShape(RoundedRectangle(cornerRadius: 14))
    }
}
