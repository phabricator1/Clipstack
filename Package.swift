// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "ClipStack",
    platforms: [.macOS(.v12)],
    targets: [
        .executableTarget(
            name: "ClipStack",
            path: "Sources/ClipStack"
        )
    ]
)
