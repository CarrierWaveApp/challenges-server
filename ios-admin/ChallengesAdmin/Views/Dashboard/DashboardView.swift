import Charts
import SwiftUI

struct DashboardView: View {
    @EnvironmentObject var config: ServerConfig
    @State private var health: HealthResponse?
    @State private var challenges: ChallengesListResponse?
    @State private var potaStatus: PotaSyncStatusResponse?
    @State private var boundaryStatus: ParkBoundariesStatusResponse?
    @State private var trailStatus: TrailStatusResponse?
    @State private var spotsResponse: SpotsListResponse?
    @State private var clubs: [ClubAdminResponse]?
    @State private var adminStats: AdminStatsResponse?
    @State private var userCountsByHour: [UserCountByHour]?
    @State private var error: String?
    @State private var serverDown = false
    @State private var isLoading = true
    @State private var lastRefresh: Date?

    private var api: APIClient { APIClient(config: config) }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 16) {
                    if isLoading && health == nil && !serverDown {
                        ProgressView("Connecting to server...")
                            .padding(.top, 40)
                    } else if serverDown {
                        serverDownSection
                    } else {
                        if let error {
                            ErrorBanner(message: error)
                        }
                        serverStatusSection
                        countsSection
                        usersSection
                        userGrowthChartSection
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

    // MARK: - Server Down

    private var serverDownSection: some View {
        VStack(spacing: 16) {
            Spacer().frame(height: 40)

            Image(systemName: "exclamationmark.icloud.fill")
                .font(.system(size: 56))
                .foregroundStyle(.red)

            Text("Server Unreachable")
                .font(.title2.bold())

            Text(config.baseURL)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fontDesign(.monospaced)

            if let error {
                Text(error)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)
            }

            Button {
                Task { await loadAll() }
            } label: {
                Label("Retry", systemImage: "arrow.clockwise")
                    .padding(.horizontal, 20)
                    .padding(.vertical, 10)
            }
            .buttonStyle(.borderedProminent)
            .tint(.blue)
            .padding(.top, 8)
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

    // MARK: - Users

    private var usersSection: some View {
        VStack(spacing: 12) {
            HStack {
                Text("Users")
                    .font(.headline)
                Spacer()
            }

            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible()),
                GridItem(.flexible()),
            ], spacing: 12) {
                CountCard(
                    title: "Total",
                    count: adminStats?.totalUsers,
                    icon: "person.crop.circle.fill",
                    color: .blue
                )
                CountCard(
                    title: "Last 7 Days",
                    count: adminStats?.usersLast7Days,
                    icon: "calendar.badge.plus",
                    color: .green
                )
                CountCard(
                    title: "Last 30 Days",
                    count: adminStats?.usersLast30Days,
                    icon: "calendar",
                    color: .teal
                )
            }
        }
        .cardStyle()
    }

    // MARK: - User Growth Chart

    private var userGrowthChartSection: some View {
        VStack(spacing: 12) {
            HStack {
                Text("Active Users per Hour")
                    .font(.headline)
                Spacer()
                Text("Last 30 Days")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if let data = userCountsByHour, !data.isEmpty {
                Chart(data) { point in
                    BarMark(
                        x: .value("Hour", point.hour),
                        y: .value("Users", point.count)
                    )
                    .foregroundStyle(.blue.gradient)
                }
                .chartXAxis {
                    AxisMarks(values: .stride(by: .day, count: 7)) { _ in
                        AxisGridLine()
                        AxisValueLabel(format: .dateTime.month(.abbreviated).day())
                    }
                }
                .chartYAxis {
                    AxisMarks { _ in
                        AxisGridLine()
                        AxisValueLabel()
                    }
                }
                .frame(height: 200)
            } else {
                Text("No data available")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .frame(height: 200)
                    .frame(maxWidth: .infinity)
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

            if let boundaries = boundaryStatus {
                AggregatorRow(
                    name: "Park Boundaries",
                    progress: Double(boundaries.completionPercentage) / 100.0,
                    detail: "\(boundaries.totalCached)/\(boundaries.totalParks) parks"
                )
            }

            if let trails = trailStatus {
                AggregatorRow(
                    name: "Historic Trails",
                    progress: Double(trails.completionPercentage) / 100.0,
                    detail: "\(trails.totalCached)/\(trails.totalCatalog) trails"
                )
            }
        }
        .cardStyle()
    }

    // MARK: - Data Loading

    private func loadAll() async {
        isLoading = true
        error = nil
        serverDown = false

        await withTaskGroup(of: Void.self) { group in
            group.addTask { await loadHealth() }
            group.addTask { await loadChallenges() }
            group.addTask { await loadPotaStatus() }
            group.addTask { await loadBoundaryStatus() }
            group.addTask { await loadTrailStatus() }
            group.addTask { await loadClubs() }
            group.addTask { await loadAdminStats() }
            group.addTask { await loadUserCountsByHour() }
        }

        lastRefresh = Date()
        isLoading = false
    }

    private func loadHealth() async {
        do {
            health = try await api.getHealth()
            serverDown = false
        } catch {
            if !error.isCancellation {
                serverDown = true
                self.error = error.localizedDescription
            }
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

    private func loadBoundaryStatus() async {
        do {
            boundaryStatus = try await api.getParkBoundariesStatus()
        } catch {
            // Non-critical
        }
    }

    private func loadTrailStatus() async {
        do {
            trailStatus = try await api.getTrailStatus()
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

    private func loadAdminStats() async {
        do {
            adminStats = try await api.getAdminStats()
        } catch {
            // Non-critical
        }
    }

    private func loadUserCountsByHour() async {
        do {
            userCountsByHour = try await api.getUserCountsByHour()
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
