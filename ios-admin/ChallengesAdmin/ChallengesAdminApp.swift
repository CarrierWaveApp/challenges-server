import SwiftUI

@main
struct ChallengesAdminApp: App {
    @StateObject private var serverConfig = ServerConfig()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(serverConfig)
        }
    }
}
