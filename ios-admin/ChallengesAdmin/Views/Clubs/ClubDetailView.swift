import SwiftUI

struct ClubDetailView: View {
    @EnvironmentObject var config: ServerConfig
    @State private var club: ClubAdminResponse

    @State private var members: [ClubMemberAdminResponse] = []
    @State private var isLoading = true
    @State private var error: String?
    @State private var showingAddMember = false
    @State private var showingEditSheet = false

    private var api: APIClient { APIClient(config: config) }

    init(club: ClubAdminResponse) {
        _club = State(initialValue: club)
    }

    var body: some View {
        List {
            clubInfoSection
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
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
            }
        }
        .refreshable { await loadMembers() }
        .task { await loadMembers() }
        .sheet(isPresented: $showingAddMember) {
            AddMemberSheet(clubId: club.id) { await loadMembers() }
        }
        .sheet(isPresented: $showingEditSheet) {
            EditClubSheet(club: club) { updated in
                club = updated
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

    private func loadMembers() async {
        isLoading = true
        do {
            members = try await api.getClubMembers(clubId: club.id)
        } catch {
            self.error = error.localizedDescription
        }
        isLoading = false
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
