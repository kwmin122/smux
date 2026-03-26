// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "SmuxApp",
    platforms: [
        .macOS(.v14),
    ],
    targets: [
        .executableTarget(
            name: "SmuxApp",
            dependencies: [
                "GhosttyKit",
                "CPtyHelper",
            ],
            path: "Sources/SmuxApp",
            linkerSettings: [
                .linkedFramework("AppKit"),
                .linkedFramework("Carbon"),
                .linkedFramework("Metal"),
                .linkedFramework("MetalKit"),
                .linkedFramework("QuartzCore"),
                .linkedFramework("WebKit"),
                .linkedLibrary("c++"),
            ]
        ),
        .target(
            name: "CPtyHelper",
            path: "Sources/CPtyHelper",
            publicHeadersPath: "include"
        ),
        .binaryTarget(
            name: "GhosttyKit",
            path: "Frameworks/GhosttyKit.xcframework"
        ),
    ]
)
