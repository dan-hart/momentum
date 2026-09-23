// swift-tools-version: 6.0
// SPDX-License-Identifier: GPL-3.0-or-later
import PackageDescription

let package = Package(
    name: "MomentumMobile",
    platforms: [.iOS("26.0"), .macOS("26.0")],
    products: [.library(name: "MomentumMobile", targets: ["MomentumMobile"])],
    dependencies: [
        .package(path: "../../../macos/Packages/MomentumCore"),
        .package(path: "../../../macos/Packages/MomentumKit"),
        .package(url: "https://github.com/dan-hart/DHFlatUIColors.git", revision: "821b077fc94ba45422ded8f38ee5b532dbabfd3e")
    ],
    targets: [
        .target(name: "MomentumMobile", dependencies: [
            .product(name: "MomentumCore", package: "MomentumCore"),
            .product(name: "MomentumKit", package: "MomentumKit"),
            .product(name: "DHFlatUIColors", package: "DHFlatUIColors")
        ]),
        .testTarget(name: "MomentumMobileTests", dependencies: ["MomentumMobile"])
    ]
)
