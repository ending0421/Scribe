// swift-tools-version:5.9
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
            path: "Sources/Scribe"
        ),
        .binaryTarget(
            name: "ScribeFFI",
            url: "https://github.com/ending0421/Scribe/releases/download/v0.1.0/Scribe.xcframework.zip",
            checksum: "66b33e5040222273a76430179549fe0d4c9f7e5e1a6d2ee70f3cc83ffb4a6bf4"
        ),
    ]
)
