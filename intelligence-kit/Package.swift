// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "IntelligenceKit",
    platforms: [
        .macOS(.v26)  // Foundation Models requires macOS 26.0+
    ],
    products: [
        .executable(
            name: "IntelligenceKit",
            targets: ["IntelligenceKit"]
        )
    ],
    targets: [
        .executableTarget(
            name: "IntelligenceKit",
            swiftSettings: [
                .enableUpcomingFeature("StrictConcurrency")
            ]
        ),
        .testTarget(
            name: "IntelligenceKitTests",
            dependencies: ["IntelligenceKit"]
        )
    ]
)
