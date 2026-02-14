// swift-tools-version: 6.2
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "JarvisListen",
    platforms: [
        .macOS(.v15)
    ],
    targets: [
        .executableTarget(
            name: "JarvisListen",
            swiftSettings: [
                .enableUpcomingFeature("StrictConcurrency")
            ]
        ),
        .testTarget(
            name: "JarvisListenTests",
            dependencies: ["JarvisListen"]
        ),
    ]
)
