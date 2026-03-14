import SwiftUI

struct ContentView: View {
    @EnvironmentObject var config: ServerConfig

    var body: some View {
        Group {
            if config.isConfigured {
                MainTabView()
            } else {
                ServerSetupView()
            }
        }
    }
}

struct MainTabView: View {
    var body: some View {
        TabView {
            DashboardView()
                .tabItem {
                    Label("Dashboard", systemImage: "gauge.open.with.lines.needle.33percent")
                }

            AggregatorsView()
                .tabItem {
                    Label("Aggregators", systemImage: "arrow.triangle.2.circlepath")
                }

            ProgramsListView()
                .tabItem {
                    Label("Programs", systemImage: "list.bullet")
                }

            EventsListView()
                .tabItem {
                    Label("Events", systemImage: "calendar")
                }

            ClubsListView()
                .tabItem {
                    Label("Clubs", systemImage: "person.3")
                }

            ChallengesListView()
                .tabItem {
                    Label("Challenges", systemImage: "trophy")
                }

            SettingsView()
                .tabItem {
                    Label("Settings", systemImage: "gear")
                }
        }
    }
}
