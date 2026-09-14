// swift-tools-version: 6.0
// The Swift package lives in packages/swift. This manifest sits at the repo
// root because SwiftPM resolves a package from a repository root:
//   .package(url: "https://github.com/spotta85/anyagent", from: "0.0.5")

import PackageDescription

let package = Package(
    name: "Anyagent",
    platforms: [.macOS(.v13)],
    products: [.library(name: "Anyagent", targets: ["Anyagent"])],
    targets: [
        .target(name: "Anyagent", path: "packages/swift/Sources/Anyagent"),
        .testTarget(name: "AnyagentTests", dependencies: ["Anyagent"], path: "packages/swift/Tests/AnyagentTests"),
    ]
)
