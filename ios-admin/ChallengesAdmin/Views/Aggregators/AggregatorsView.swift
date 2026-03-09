import SwiftUI

struct AggregatorsView: View {
    @EnvironmentObject var config: ServerConfig
    @State private var potaStatus: PotaSyncStatusResponse?
    @State private var boundaryStatus: ParkBoundariesStatusResponse?
    @State private var programs: ProgramListResponse?
    @State private var error: String?
    @State private var isLoading = true

    private var api: APIClient { APIClient(config: config) }

    var body: some View {
        NavigationStack {
            List {
                if let error {
                    Section {
                        ErrorBanner(message: error)
                    }
                    .listRowBackground(Color.clear)
                    .listRowInsets(EdgeInsets())
                }

                if let pota = potaStatus {
                    potaSection(pota)
                }

                if let boundaries = boundaryStatus {
                    boundariesSection(boundaries)
                }

                spotAggregatorSection

                if let programs {
                    programsSection(programs)
                }
            }
            .navigationTitle("Aggregators")
            .refreshable { await loadAll() }
            .task { await loadAll() }
            .overlay {
                if isLoading && potaStatus == nil {
                    ProgressView()
                }
            }
        }
    }

    // MARK: - POTA Stats

    private func potaSection(_ pota: PotaSyncStatusResponse) -> some View {
        Section {
            LabeledContent("Completion") {
                Text(String(format: "%.1f%%", pota.completionPercentage))
                    .fontDesign(.monospaced)
                    .foregroundStyle(pota.completionPercentage >= 100 ? .green : .primary)
            }

            ProgressView(value: min(pota.completionPercentage / 100.0, 1.0))
                .tint(pota.completionPercentage >= 100 ? .green : .blue)

            LabeledContent("Total Parks") {
                Text("\(pota.totalParks)")
                    .fontDesign(.monospaced)
            }

            LabeledContent("Fetched") {
                Text("\(pota.fetchedParks)")
                    .fontDesign(.monospaced)
                    .foregroundStyle(.green)
            }

            LabeledContent("Remaining") {
                Text("\(pota.unfetchedParks)")
                    .fontDesign(.monospaced)
                    .foregroundStyle(pota.unfetchedParks > 0 ? .orange : .green)
            }

            if let oldest = pota.oldestFetch {
                LabeledContent("Oldest Fetch") {
                    Text(oldest.formatted(.relative(presentation: .named)))
                        .foregroundStyle(.secondary)
                }
            }

            if let newest = pota.newestFetch {
                LabeledContent("Newest Fetch") {
                    Text(newest.formatted(.relative(presentation: .named)))
                        .foregroundStyle(.secondary)
                }
            }
        } header: {
            Label("POTA Stats Aggregator", systemImage: "tree.fill")
        }
    }

    // MARK: - Park Boundaries

    private func boundariesSection(_ boundaries: ParkBoundariesStatusResponse) -> some View {
        Section {
            LabeledContent("Completion") {
                Text(String(format: "%.1f%%", boundaries.completionPercentage))
                    .fontDesign(.monospaced)
                    .foregroundStyle(boundaries.completionPercentage >= 100 ? .green : .primary)
            }

            ProgressView(value: min(boundaries.completionPercentage / 100.0, 1.0))
                .tint(boundaries.completionPercentage >= 100 ? .green : .blue)

            LabeledContent("Total Parks") {
                Text("\(boundaries.totalParks)")
                    .fontDesign(.monospaced)
            }

            LabeledContent("Fetched") {
                Text("\(boundaries.fetchedParks)")
                    .fontDesign(.monospaced)
                    .foregroundStyle(.green)
            }

            LabeledContent("Remaining") {
                Text("\(boundaries.unfetchedParks)")
                    .fontDesign(.monospaced)
                    .foregroundStyle(boundaries.unfetchedParks > 0 ? .orange : .green)
            }

            if let errorCount = boundaries.errorCount, errorCount > 0 {
                LabeledContent("Errors") {
                    Text("\(errorCount)")
                        .fontDesign(.monospaced)
                        .foregroundStyle(.red)
                }
            }

            if let oldest = boundaries.oldestFetch {
                LabeledContent("Oldest Fetch") {
                    Text(oldest.formatted(.relative(presentation: .named)))
                        .foregroundStyle(.secondary)
                }
            }

            if let newest = boundaries.newestFetch {
                LabeledContent("Newest Fetch") {
                    Text(newest.formatted(.relative(presentation: .named)))
                        .foregroundStyle(.secondary)
                }
            }
        } header: {
            Label("Park Boundaries Aggregator", systemImage: "map.fill")
        }
    }

    // MARK: - Spot Aggregators

    private var spotAggregatorSection: some View {
        Section {
            SpotAggregatorRow(name: "POTA Spots", icon: "tree", interval: "60s", source: "api.pota.app")
            SpotAggregatorRow(name: "RBN", icon: "antenna.radiowaves.left.and.right", interval: "30s", source: "vailrerbn.com")
            SpotAggregatorRow(name: "SOTA Spots", icon: "mountain.2", interval: "90s", source: "api2.sota.org.uk")
            SpotAggregatorRow(name: "TTL Cleanup", icon: "trash", interval: "120s", source: "Removes expired spots")
        } header: {
            Label("Spot Aggregators", systemImage: "dot.radiowaves.left.and.right")
        } footer: {
            Text("Spot aggregators run continuously when enabled in server config.")
        }
    }

    // MARK: - Programs

    private func programsSection(_ programs: ProgramListResponse) -> some View {
        Section {
            ForEach(programs.programs) { program in
                HStack {
                    VStack(alignment: .leading) {
                        Text(program.name)
                            .font(.subheadline.weight(.medium))
                        if let caps = program.capabilities {
                            Text(caps.joined(separator: ", "))
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                                .lineLimit(1)
                        }
                    }
                    Spacer()
                    if program.isActive == true {
                        Text("Active")
                            .font(.caption)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(.green.opacity(0.15))
                            .foregroundStyle(.green)
                            .clipShape(Capsule())
                    } else {
                        Text("Inactive")
                            .font(.caption)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 2)
                            .background(.secondary.opacity(0.15))
                            .foregroundStyle(.secondary)
                            .clipShape(Capsule())
                    }
                }
            }
        } header: {
            Label("Programs (\(programs.programs.count))", systemImage: "list.bullet")
        }
    }

    // MARK: - Loading

    private func loadAll() async {
        isLoading = true
        error = nil

        await withTaskGroup(of: Void.self) { group in
            group.addTask {
                do { potaStatus = try await api.getPotaSyncStatus() } catch { self.error = error.localizedDescription }
            }
            group.addTask {
                do { boundaryStatus = try await api.getParkBoundariesStatus() } catch {}
            }
            group.addTask {
                do { programs = try await api.getPrograms() } catch {}
            }
        }

        isLoading = false
    }
}

struct SpotAggregatorRow: View {
    let name: String
    let icon: String
    let interval: String
    let source: String

    var body: some View {
        HStack {
            Image(systemName: icon)
                .foregroundStyle(.blue)
                .frame(width: 24)
            VStack(alignment: .leading, spacing: 2) {
                Text(name)
                    .font(.subheadline.weight(.medium))
                Text(source)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Text("every \(interval)")
                .font(.caption)
                .foregroundStyle(.secondary)
                .fontDesign(.monospaced)
        }
    }
}
