// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Scribe",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
        .library(
            name: "Scribe",
            targets: ["Scribe"]
        ),
    ],
    targets: [
        .target(
            name: "Scribe",
            dependencies: ["ScribeFFI"],
            path: "Sources/Scribe",
            swiftSettings: [
                .enableUpcomingFeature("BareSlashRegexLiterals"),
                .enableUpcomingFeature("ConciseMagicFile"),
                .enableUpcomingFeature("ExistentialAny"),
                .enableUpcomingFeature("ForwardTrailingClosures"),
                .enableUpcomingFeature("ImplicitOpenExistentials"),
                .enableUpcomingFeature("StrictConcurrency")
            ]
        ),
        .binaryTarget(
            name: "ScribeFFI",
            path: "ScribeFFI.xcframework"
        ),
        .testTarget(
            name: "ScribeTests",
            dependencies: ["Scribe"]
        ),
    ]
)
