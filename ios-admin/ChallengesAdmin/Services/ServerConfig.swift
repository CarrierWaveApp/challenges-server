import Foundation

class ServerConfig: ObservableObject {
    @Published var serverURL: String {
        didSet { UserDefaults.standard.set(serverURL, forKey: "serverURL") }
    }
    @Published var adminToken: String {
        didSet {
            // In production, use Keychain instead of UserDefaults
            UserDefaults.standard.set(adminToken, forKey: "adminToken")
        }
    }

    var isConfigured: Bool {
        !serverURL.isEmpty && !adminToken.isEmpty
    }

    var baseURL: String {
        let url = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
        return url.hasSuffix("/") ? String(url.dropLast()) : url
    }

    init() {
        self.serverURL = UserDefaults.standard.string(forKey: "serverURL") ?? ""
        self.adminToken = UserDefaults.standard.string(forKey: "adminToken") ?? ""
    }

    func disconnect() {
        serverURL = ""
        adminToken = ""
    }
}
