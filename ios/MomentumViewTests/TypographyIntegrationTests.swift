// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import MomentumMobile
import SwiftUI
import UIKit
import XCTest

@MainActor final class TypographyIntegrationTests: XCTestCase {
    func testBundledFacesAreAvailableAndUnknownFontFallsBack() {
        for name in ["AtkinsonHyperlegibleMono-Regular", "AtkinsonHyperlegibleNext-Regular", "OpenDyslexic-Regular"] {
            XCTAssertEqual(MobileTypography.resolvedName(name), name)
            for suffix in ["Regular", "Bold", "Italic", "BoldItalic"] {
                let face = name.replacingOccurrences(of: "Regular", with: suffix)
                XCTAssertNotNil(UIFont(name: face, size: 17))
            }
        }
        XCTAssertNil(MobileTypography.resolvedName(""))
        XCTAssertNil(MobileTypography.resolvedName("Momentum-Missing-Font"))
    }

    func testActualSwiftUITextRetainsIndependentScalesAndDynamicTypeForEveryFamily() throws {
        for choice in BundledAppFont.allCases {
            let task = Text("Grüße — task notes 0123").mobileFont(content: true)
            let baseline = try HostingFixture.render(task, width: 800, fontName: choice.rawValue)
            let content = try HostingFixture.render(task, width: 800, contentScale: 1.6, fontName: choice.rawValue)
            let interfaceOnly = try HostingFixture.render(task, width: 800, interfaceScale: 1.6, fontName: choice.rawValue)
            let accessible = try HostingFixture.render(task, width: 800, size: .accessibility3, fontName: choice.rawValue)
            XCTAssertGreaterThan(content.size.height, baseline.size.height, choice.label)
            XCTAssertEqual(interfaceOnly.size.height, baseline.size.height, accuracy: 0.1, choice.label)
            XCTAssertGreaterThan(accessible.size.height, content.size.height, choice.label)
            let label = Text("Einstellungen").mobileFont(.caption)
            let labelBase = try HostingFixture.render(label, fontName: choice.rawValue)
            let labelContent = try HostingFixture.render(label, contentScale: 1.6, fontName: choice.rawValue)
            let labelInterface = try HostingFixture.render(label, interfaceScale: 1.6, fontName: choice.rawValue)
            XCTAssertEqual(labelBase.size.height, labelContent.size.height, accuracy: 0.1, choice.label)
            XCTAssertGreaterThan(labelInterface.size.height, labelBase.size.height, choice.label)
        }
    }

    func testMonoReducesCharacterSpacingAtTheOriginalPointSize() throws {
        let sample = "Make room for what matters"
        let name = BundledAppFont.atkinsonMono.rawValue
        let original = try HostingFixture.render(
            Text(sample).font(.custom(name, size: MobileTextStyle.body.baseSize, relativeTo: .body)),
            width: 600, fontName: name)
        for isTaskText in [false, true] {
            let rendered = try HostingFixture.render(
                Text(sample).mobileFont(content: isTaskText), width: 600, fontName: name)
            XCTAssertEqual(rendered.size.height, original.size.height, accuracy: 0.1)
            XCTAssertLessThan(rendered.size.width, original.size.width)
        }
    }

    func testNativeNavigationTitlesUpdateAndResetWithAppearancePreference() async throws {
        let name = "momentum-navigation-font-test-\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defaults.set(BundledAppFont.atkinsonMono.rawValue, forKey: PrefKey.appFontName)
        let view = NavigationStack {
            List { Text("Search tasks, projects, tags, and notes.") }
                .navigationTitle("Search")
                .momentumNavigationCanvas()
        }
        let mounted = try HostingFixture.mount(view, defaults: defaults, defaultsName: name)
        defer { mounted.unmount() }

        let bar = try XCTUnwrap(navigationBar(in: mounted.controller))
        func titleFonts() -> [UIFont] {
            [bar.standardAppearance.titleTextAttributes, bar.standardAppearance.largeTitleTextAttributes]
                .compactMap { $0[.font] as? UIFont }
        }
        try await mounted.wait(until: "Mono in both native title sizes") {
            titleFonts().map(\.fontName) == Array(repeating: "AtkinsonHyperlegibleMono-Bold", count: 2)
        }
        XCTAssertEqual(titleFonts().map { Double($0.pointSize) }, [MobileTextStyle.headline.baseSize, MobileTextStyle.largeTitle.baseSize])
        XCTAssertLessThan(bar.standardAppearance.titleTextAttributes[.kern] as? CGFloat ?? 0, 0)
        XCTAssertLessThan(bar.standardAppearance.largeTitleTextAttributes[.kern] as? CGFloat ?? 0, 0)
        let initialSize = try XCTUnwrap(titleFonts().first).pointSize
        defaults.set(1.3, forKey: MobileAppearance.interfaceScaleKey)
        try await mounted.wait(until: "navigation title follows interface size") {
            (titleFonts().first?.pointSize ?? 0) > initialSize
        }
        defaults.set(BundledAppFont.atkinsonNext.rawValue, forKey: PrefKey.appFontName)
        try await mounted.wait(until: "Next in both native title sizes") {
            titleFonts().map(\.fontName) == Array(repeating: "AtkinsonHyperlegibleNext-Bold", count: 2)
        }
        XCTAssertNil(bar.standardAppearance.titleTextAttributes[.kern])
        XCTAssertNil(bar.standardAppearance.largeTitleTextAttributes[.kern])
        defaults.set("", forKey: PrefKey.appFontName)
        defaults.set(1.0, forKey: MobileAppearance.interfaceScaleKey)
        try await mounted.wait(until: "native defaults restored") {
            titleFonts().allSatisfy { $0.familyName == UIFont.systemFont(ofSize: 17).familyName }
        }
    }

    private func navigationBar(in controller: UIViewController) -> UINavigationBar? {
        if let navigation = controller as? UINavigationController { return navigation.navigationBar }
        return controller.children.lazy.compactMap { self.navigationBar(in: $0) }.first
    }

    func testSemanticHeadingsAndCaptionsUseSelectedFont() throws {
        let sample = VStack(alignment: .leading) {
            Text("Make room for what matters").mobileFont(.title2).bold()
            Text("Grüße — 0123456789").mobileFont()
            Text("Task details and notes").mobileFont(.caption)
        }
        let system = try HostingFixture.render(sample, width: 390)
        for choice in BundledAppFont.allCases.filter { $0 != .system } {
            let rendered = try HostingFixture.render(sample, width: 390, fontName: choice.rawValue)
            XCTAssertNotEqual(rendered.image.pngData(), system.image.pngData())
            let attachment = XCTAttachment(image: rendered.image)
            attachment.name = choice.label
            attachment.lifetime = .keepAlways
            add(attachment)
        }
    }
}
