// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import SwiftUI
import UIKit
import XCTest

@MainActor final class TypographyIntegrationTests: XCTestCase {
    func testBundledFacesAreAvailableAndUnknownFontFallsBack() {
        for name in ["AtkinsonHyperlegibleMono-Regular", "OpenDyslexic-Regular"] {
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

    func testSemanticHeadingsAndCaptionsUseSelectedFont() throws {
        let sample = VStack(alignment: .leading) {
            Text("Make room for what matters").mobileFont(.title2).bold()
            Text("Grüße — 0123456789").mobileFont()
            Text("Task details and notes").mobileFont(.caption)
        }
        let system = try HostingFixture.render(sample, width: 390)
        for choice in [BundledAppFont.atkinsonMono, .openDyslexic] {
            let rendered = try HostingFixture.render(sample, width: 390, fontName: choice.rawValue)
            XCTAssertNotEqual(rendered.image.pngData(), system.image.pngData())
            let attachment = XCTAttachment(image: rendered.image)
            attachment.name = choice.label
            attachment.lifetime = .keepAlways
            add(attachment)
        }
    }
}
