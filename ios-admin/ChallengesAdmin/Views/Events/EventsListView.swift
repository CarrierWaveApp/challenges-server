import SwiftUI

struct EventsListView: View {
    @EnvironmentObject var config: ServerConfig
    @State private var events: [EventListItem] = []
    @State private var isLoading = true
    @State private var error: String?
    @State private var selectedFilter: EventFilter = .pending
    @State private var eventToDelete: EventListItem?

    private var api: APIClient { APIClient(config: config) }

    var body: some View {
        NavigationStack {
            List {
                if let error {
                    ErrorBanner(message: error)
                        .listRowBackground(Color.clear)
                        .listRowInsets(EdgeInsets())
                }

                filterSection

                ForEach(events) { event in
                    NavigationLink(destination: EventDetailView(eventId: event.id)) {
                        EventRow(event: event)
                    }
                    .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                        Button(role: .destructive) {
                            eventToDelete = event
                        } label: {
                            Label("Delete", systemImage: "trash")
                        }
                    }
                }
            }
            .navigationTitle("Events")
            .refreshable { await loadEvents() }
            .task { await loadEvents() }
            .overlay {
                if isLoading && events.isEmpty {
                    ProgressView()
                } else if events.isEmpty && error == nil {
                    ContentUnavailableView(
                        "No Events",
                        systemImage: "calendar",
                        description: Text("No \(selectedFilter.label.lowercased()) events found.")
                    )
                }
            }
            .alert("Delete Event?", isPresented: .init(
                get: { eventToDelete != nil },
                set: { if !$0 { eventToDelete = nil } }
            )) {
                Button("Cancel", role: .cancel) { eventToDelete = nil }
                Button("Delete", role: .destructive) {
                    if let event = eventToDelete {
                        Task { await deleteEvent(event) }
                    }
                }
            } message: {
                if let event = eventToDelete {
                    Text("This will permanently delete \"\(event.name)\".")
                }
            }
            .onChange(of: selectedFilter) {
                Task { await loadEvents() }
            }
        }
    }

    private var filterSection: some View {
        Section {
            Picker("Status", selection: $selectedFilter) {
                ForEach(EventFilter.allCases) { filter in
                    Text(filter.label).tag(filter)
                }
            }
            .pickerStyle(.segmented)
        }
        .listRowBackground(Color.clear)
        .listRowInsets(EdgeInsets(top: 0, leading: 16, bottom: 0, trailing: 16))
    }

    private func loadEvents() async {
        isLoading = true
        error = nil
        do {
            let response = try await api.getEvents(
                status: selectedFilter.apiValue,
                limit: 100
            )
            events = response.events
        } catch {
            if !error.isCancellation {
                self.error = error.localizedDescription
            }
        }
        isLoading = false
    }

    private func deleteEvent(_ event: EventListItem) async {
        do {
            try await api.deleteEvent(id: event.id)
            events.removeAll { $0.id == event.id }
        } catch {
            self.error = error.localizedDescription
        }
        eventToDelete = nil
    }
}

// MARK: - Event Filter

enum EventFilter: String, CaseIterable, Identifiable {
    case pending
    case approved
    case rejected
    case all

    var id: String { rawValue }

    var label: String {
        switch self {
        case .pending: return "Pending"
        case .approved: return "Approved"
        case .rejected: return "Rejected"
        case .all: return "All"
        }
    }

    var apiValue: String? {
        switch self {
        case .all: return nil
        default: return rawValue
        }
    }
}

// MARK: - Event Row

struct EventRow: View {
    let event: EventListItem

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(event.name)
                    .font(.headline)
                Spacer()
                EventStatusBadge(status: event.status)
            }

            HStack(spacing: 8) {
                EventTypeBadge(eventType: event.eventType)

                if let venue = event.venueName, !venue.isEmpty {
                    Text(venue)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
            }

            HStack {
                Label(locationText, systemImage: "mappin")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                Spacer()

                Label(event.startDate.formatted(date: .abbreviated, time: .shortened), systemImage: "calendar")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            HStack {
                Text("by \(event.submittedBy)")
                    .font(.caption)
                    .foregroundStyle(.blue)
                    .fontDesign(.monospaced)

                Spacer()

                Text(event.createdAt.formatted(.relative(presentation: .named)))
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.vertical, 4)
    }

    private var locationText: String {
        if let state = event.state, !state.isEmpty {
            return "\(event.city), \(state)"
        }
        return "\(event.city), \(event.country)"
    }
}

// MARK: - Badges

struct EventStatusBadge: View {
    let status: String

    var color: Color {
        switch status {
        case "pending": return .orange
        case "approved": return .green
        case "rejected": return .red
        default: return .secondary
        }
    }

    var body: some View {
        Text(status.capitalized)
            .font(.caption)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(color.opacity(0.15))
            .foregroundStyle(color)
            .clipShape(Capsule())
    }
}

struct EventTypeBadge: View {
    let eventType: String

    var label: String {
        switch eventType {
        case "club_meeting": return "Club Meeting"
        case "swap_meet": return "Swap Meet"
        case "field_day": return "Field Day"
        case "special_event": return "Special Event"
        case "hamfest": return "Hamfest"
        case "net": return "Net"
        default: return "Other"
        }
    }

    var body: some View {
        Text(label)
            .font(.caption)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(.blue.opacity(0.1))
            .foregroundStyle(.blue)
            .clipShape(Capsule())
    }
}
