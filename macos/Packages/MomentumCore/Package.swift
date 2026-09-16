// swift-tools-version: 6.0
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//
// The Rust core as a Swift package: the UniFFI-generated `momentum.swift` over the
// static library in `momentum_ffi.xcframework`. Both files are produced by
// `macos/scripts/build-core.sh`; neither is committed.
import PackageDescription

let package = Package(
    name: "MomentumCore",
    platforms: [.macOS("26.0")],
    products: [
        .library(name: "MomentumCore", targets: ["MomentumCore"])
    ],
    targets: [
        .binaryTarget(name: "momentum_ffi", path: "momentum_ffi.xcframework"),
        .target(
            name: "MomentumCore",
            dependencies: ["momentum_ffi"],
            swiftSettings: [.swiftLanguageMode(.v5)]
        ),
    ]
)
