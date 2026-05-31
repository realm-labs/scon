// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "scon",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "SconCore", targets: ["SconCore"])
    ],
    targets: [
        .target(name: "SconCore"),
        .testTarget(name: "SconCoreTests", dependencies: ["SconCore"])
    ]
)
