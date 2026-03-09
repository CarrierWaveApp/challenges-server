// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ChallengesAdmin",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "ChallengesAdmin", targets: ["ChallengesAdmin"]),
    ],
    targets: [
        .target(
            name: "ChallengesAdmin",
            path: "ChallengesAdmin"
        ),
    ]
)
