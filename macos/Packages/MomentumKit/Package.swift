// swift-tools-version: 6.0
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// Shared Apple presentation support: wording, task forms, preferences and Keychain.
// The macOS state object and desktop services are available only on macOS.
//
// It lives in a package so the unit tests run against it directly — no app to launch, no
// window to drive, no UI automation. `swift test` here finishes in about a second, and
// Xcode lists the same tests in its test navigator.
import PackageDescription

let package = Package(
    name: "MomentumKit",
    defaultLocalization: "en",
    platforms: [.macOS("26.0"), .iOS("26.0")],
    products: [
        .library(name: "MomentumKit", targets: ["MomentumKit"])
    ],
    dependencies: [
        .package(path: "../MomentumCore")
    ],
    targets: [
        .target(name: "MomentumKit", dependencies: [.product(name: "MomentumCore", package: "MomentumCore")],
                resources: [.process("Resources")]),
        .testTarget(name: "MomentumKitTests", dependencies: ["MomentumKit"]),
    ]
)
