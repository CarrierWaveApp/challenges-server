import SwiftUI

struct ClubDetailView: View {
    @EnvironmentObject var config: ServerConfig
    @State private var club: ClubAdminResponse

    @State private var members: [ClubMemberAdminResponse] = []
    @State private var monitors: [MembershipMonitorResponse] = []
    @State private var isLoading = true
    @State private var error: String?
    @State private var showingAddMember = false
    @State private var showingEditSheet = false
    @State private var showingAddMonitor = false
    @State private var isImporting = false
    @State private var importResult: String?
    @State private var checkResult: String?

    private var api: APIClient { APIClient(config: config) }

    init(club: ClubAdminResponse) {
        _club = State(initialValue: club)
    }

    var body: some View {
        List {
            clubInfoSection
            monitorsSection
            membersSection
        }
        .navigationTitle(club.name)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Menu {
                    Button {
                        showingEditSheet = true
                    } label: {
                        Label("Edit Club", systemImage: "pencil")
                    }
                    Button {
                        showingAddMember = true
                    } label: {
                        Label("Add Member", systemImage: "person.badge.plus")
                    }
                    Button {
                        showingAddMonitor = true
                    } label: {
                        Label("Add Monitor", systemImage: "antenna.radiowaves.left.and.right")
                    }
                    if club.notesUrl != nil {
                        Button {
                            Task { await importFromNotes() }
                        } label: {
                            Label("Import from Notes", systemImage: "square.and.arrow.down")
                        }
                        .disabled(isImporting)
                    }
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
            }
        }
        .refreshable { await loadData() }
        .task { await loadData() }
        .sheet(isPresented: $showingAddMember) {
            AddMemberSheet(clubId: club.id) { await loadMembers() }
        }
        .sheet(isPresented: $showingEditSheet) {
            EditClubSheet(club: club) { updated in
                club = updated
            }
        }
        .sheet(isPresented: $showingAddMonitor) {
            AddMonitorSheet(clubId: club.id) { await loadMonitors() }
        }
        .alert("Import Notes", isPresented: .init(
            get: { importResult != nil },
            set: { if !$0 { importResult = nil } }
        )) {
            Button("OK") { importResult = nil }
        } message: {
            if let result = importResult {
                Text(result)
            }
        }
        .alert("Monitor Check", isPresented: .init(
            get: { checkResult != nil },
            set: { if !$0 { checkResult = nil } }
        )) {
            Button("OK") { checkResult = nil }
        } message: {
            if let result = checkResult {
                Text(result)
            }
        }
    }

    private var clubInfoSection: some View {
        Section("Details") {
            if let callsign = club.callsign, !callsign.isEmpty {
                LabeledContent("Callsign") {
                    Text(callsign)
                        .fontDesign(.monospaced)
                }
            }
            if let desc = club.description, !desc.isEmpty {
                LabeledContent("Description") {
                    Text(desc)
                }
            }
            LabeledContent("Members") {
                Text("\(club.memberCount)")
                    .fontDesign(.monospaced)
            }
            if let createdAt = club.createdAt {
                LabeledContent("Created") {
                    Text(createdAt.formatted(date: .abbreviated, time: .shortened))
                }
            }
            if let notesUrl = club.notesUrl, !notesUrl.isEmpty {
                LabeledContent("Notes") {
                    Text(club.notesTitle ?? notesUrl)
                        .foregroundStyle(.blue)
                }
            }
        }
    }

    private var monitorsSection: some View {
        Section("Monitors (\(monitors.count))") {
            if monitors.isEmpty && !isLoading {
                Text("No membership monitors")
                    .foregroundStyle(.secondary)
                    .font(.subheadline)
            }

            ForEach(monitors) { monitor in
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(monitor.label ?? monitor.url)
                            .font(.headline)
                            .lineLimit(1)
                        Spacer()
                        if monitor.enabled {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundStyle(.green)
                                .font(.caption)
                        } else {
                            Image(systemName: "pause.circle.fill")
                                .foregroundStyle(.secondary)
                                .font(.caption)
                        }
                    }

                    if monitor.label != nil {
                        Text(monitor.url)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }

                    HStack(spacing: 12) {
                        Label("\(monitor.intervalHours)h", systemImage: "clock")
                        if let count = monitor.lastMemberCount {
                            Label("\(count)", systemImage: "person.2")
                        }
                        if let status = monitor.lastStatus {
                            Text(status)
                                .foregroundStyle(status == "ok" ? .green : .red)
                        }
                    }
                    .font(.caption)
                    .foregroundStyle(.secondary)

                    if let checked = monitor.lastCheckedAt {
                        Text("Checked \(checked.formatted(.relative(presentation: .named)))")
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    }
                }
                .padding(.vertical, 2)
                .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                    Button(role: .destructive) {
                        Task { await deleteMonitor(monitor) }
                    } label: {
                        Label("Delete", systemImage: "trash")
                    }
                }
                .swipeActions(edge: .leading) {
                    Button {
                        Task { await triggerCheck(monitor) }
                    } label: {
                        Label("Check Now", systemImage: "arrow.clockwise")
                    }
                    .tint(.blue)
                }
            }
        }
    }

    private var membersSection: some View {
        Section("Members (\(members.count))") {
            if isLoading && members.isEmpty {
                ProgressView()
            }

            ForEach(members) { member in
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(member.callsign)
                            .font(.headline)
                            .fontDesign(.monospaced)
                        Text("Joined \(member.joinedAt.formatted(.relative(presentation: .named)))")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    RoleBadge(role: member.role)
                }
                .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                    Button(role: .destructive) {
                        Task { await removeMember(member) }
                    } label: {
                        Label("Remove", systemImage: "person.badge.minus")
                    }
                }
            }

            if let error {
                Text(error)
                    .foregroundStyle(.red)
                    .font(.caption)
            }
        }
    }

    private func loadData() async {
        await withTaskGroup(of: Void.self) { group in
            group.addTask { await loadMembers() }
            group.addTask { await loadMonitors() }
        }
    }

    private func loadMembers() async {
        isLoading = true
        do {
            members = try await api.getClubMembers(clubId: club.id)
        } catch {
            self.error = error.localizedDescription
        }
        isLoading = false
    }

    private func loadMonitors() async {
        do {
            monitors = try await api.getMonitors(clubId: club.id)
        } catch {
            if !error.isCancellation {
                self.error = error.localizedDescription
            }
        }
    }

    private func deleteMonitor(_ monitor: MembershipMonitorResponse) async {
        do {
            try await api.deleteMonitor(clubId: club.id, monitorId: monitor.id)
            monitors.removeAll { $0.id == monitor.id }
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func triggerCheck(_ monitor: MembershipMonitorResponse) async {
        do {
            let result = try await api.triggerMonitorCheck(clubId: club.id, monitorId: monitor.id)
            checkResult = "Added \(result.added), removed \(result.removed) (total: \(result.total))"
            await loadData()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func importFromNotes() async {
        isImporting = true
        do {
            let result = try await api.importNotesMembers(clubId: club.id)
            if result.imported > 0 {
                importResult = "Imported \(result.imported) member\(result.imported == 1 ? "" : "s") (\(result.skipped) already existed)"
                await loadMembers()
            } else {
                importResult = "No new members to import (\(result.skipped) already existed)"
            }
        } catch {
            self.error = error.localizedDescription
        }
        isImporting = false
    }

    private func removeMember(_ member: ClubMemberAdminResponse) async {
        do {
            try await api.removeClubMember(clubId: club.id, callsign: member.callsign)
            members.removeAll { $0.callsign == member.callsign }
        } catch {
            self.error = error.localizedDescription
        }
    }
}

struct RoleBadge: View {
    let role: String

    var color: Color {
        switch role.lowercased() {
        case "admin": return .red
        case "moderator": return .orange
        default: return .secondary
        }
    }

    var body: some View {
        Text(role.capitalized)
            .font(.caption)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(color.opacity(0.15))
            .foregroundStyle(color)
            .clipShape(Capsule())
    }
}

// MARK: - Add Member Sheet

struct AddMemberSheet: View {
    @EnvironmentObject var config: ServerConfig
    @Environment(\.dismiss) private var dismiss
    let clubId: String
    let onAdded: () async -> Void

    @State private var callsign = ""
    @State private var role = "member"
    @State private var isSubmitting = false
    @State private var error: String?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Callsign", text: $callsign)
                        .textInputAutocapitalization(.characters)
                        .fontDesign(.monospaced)

                    Picker("Role", selection: $role) {
                        Text("Member").tag("member")
                        Text("Admin").tag("admin")
                    }
                }

                if let error {
                    Section {
                        Text(error)
                            .foregroundStyle(.red)
                            .font(.subheadline)
                    }
                }
            }
            .navigationTitle("Add Member")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Add") {
                        Task { await addMember() }
                    }
                    .disabled(callsign.isEmpty || isSubmitting)
                }
            }
        }
    }

    private func addMember() async {
        isSubmitting = true
        error = nil
        let api = APIClient(config: config)
        do {
            try await api.addClubMembers(
                clubId: clubId,
                members: [AddMemberEntry(callsign: callsign.uppercased(), role: role)]
            )
            await onAdded()
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isSubmitting = false
    }
}

// MARK: - Edit Club Sheet

struct EditClubSheet: View {
    @EnvironmentObject var config: ServerConfig
    @Environment(\.dismiss) private var dismiss
    let club: ClubAdminResponse
    let onSaved: (ClubAdminResponse) -> Void

    @State private var name: String
    @State private var callsign: String
    @State private var description: String
    @State private var notesUrl: String
    @State private var notesTitle: String
    @State private var isSubmitting = false
    @State private var error: String?

    init(club: ClubAdminResponse, onSaved: @escaping (ClubAdminResponse) -> Void) {
        self.club = club
        self.onSaved = onSaved
        _name = State(initialValue: club.name)
        _callsign = State(initialValue: club.callsign ?? "")
        _description = State(initialValue: club.description ?? "")
        _notesUrl = State(initialValue: club.notesUrl ?? "")
        _notesTitle = State(initialValue: club.notesTitle ?? "")
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Name", text: $name)
                    TextField("Callsign", text: $callsign)
                        .textInputAutocapitalization(.characters)
                        .fontDesign(.monospaced)
                    TextField("Description", text: $description, axis: .vertical)
                        .lineLimit(3...6)
                }

                Section("Notes") {
                    TextField("Notes URL", text: $notesUrl)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                    TextField("Notes Title", text: $notesTitle)
                }

                if let error {
                    Section {
                        Text(error)
                            .foregroundStyle(.red)
                            .font(.subheadline)
                    }
                }
            }
            .navigationTitle("Edit Club")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        Task { await save() }
                    }
                    .disabled(name.isEmpty || isSubmitting)
                }
            }
        }
    }

    private func save() async {
        isSubmitting = true
        error = nil
        let api = APIClient(config: config)
        let request = UpdateClubRequest(
            name: name,
            callsign: callsign.isEmpty ? .null : .value(callsign),
            description: description.isEmpty ? .null : .value(description),
            notesUrl: notesUrl.isEmpty ? .null : .value(notesUrl),
            notesTitle: notesTitle.isEmpty ? .null : .value(notesTitle)
        )
        do {
            let updated = try await api.updateClub(id: club.id, request)
            onSaved(updated)
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isSubmitting = false
    }
}

// MARK: - Add Monitor Sheet

struct AddMonitorSheet: View {
    @EnvironmentObject var config: ServerConfig
    @Environment(\.dismiss) private var dismiss
    let clubId: String
    let onAdded: () async -> Void

    @State private var url = ""
    @State private var label = ""
    @State private var format = "callsign_notes"
    @State private var intervalHours = 24
    @State private var removeStale = false
    @State private var isSubmitting = false
    @State private var error: String?

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("URL", text: $url)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                    TextField("Label (optional)", text: $label)
                }

                Section("Format") {
                    Picker("Format", selection: $format) {
                        Text("Callsign Notes (Ham2K)").tag("callsign_notes")
                        Text("One Per Line").tag("one_per_line")
                    }
                    .pickerStyle(.menu)
                }

                Section("Schedule") {
                    Stepper("Every \(intervalHours) hour\(intervalHours == 1 ? "" : "s")", value: $intervalHours, in: 1...168)
                }

                Section {
                    Toggle("Remove stale members", isOn: $removeStale)
                } footer: {
                    Text("When enabled, members not found in the list will be removed from the club.")
                }

                if let error {
                    Section {
                        Text(error)
                            .foregroundStyle(.red)
                            .font(.subheadline)
                    }
                }
            }
            .navigationTitle("Add Monitor")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Add") {
                        Task { await addMonitor() }
                    }
                    .disabled(url.isEmpty || isSubmitting)
                }
            }
        }
    }

    private func addMonitor() async {
        isSubmitting = true
        error = nil
        let api = APIClient(config: config)
        let request = CreateMonitorRequest(
            url: url,
            label: label.isEmpty ? nil : label,
            format: format,
            intervalHours: intervalHours,
            removeStale: removeStale
        )
        do {
            _ = try await api.createMonitor(clubId: clubId, request)
            await onAdded()
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
        isSubmitting = false
    }
}
