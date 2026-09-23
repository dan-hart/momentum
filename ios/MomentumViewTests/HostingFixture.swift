// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumMobile
import SwiftUI
import UIKit
import XCTest

/// Public Apple APIs only. Each render owns its child controller/defaults and cleans both up.
/// Geometry is measured by SwiftUI, then UIKit captures the actual rendered content.
/// No view-tree reflection, arbitrary sleeps, UI automation or baseline recording.
@MainActor enum HostingFixture {
    // XCTest can launch an iOS scene without a key window. Keep one empty UIKit
    // window for this test process; only child controllers change between renders.
    // Keeping the scene root alive lets UIKit complete its own appearance cycle.
    private static var renderingWindow: UIWindow?

    private static func parentController() throws -> UIViewController {
        if let parent = renderingWindow?.rootViewController { return parent }
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let window = UIWindow(windowScene: scene)
        let parent = UIViewController()
        window.rootViewController = parent
        window.makeKeyAndVisible()
        renderingWindow = window
        return parent
    }

    struct Render {
        let size: CGSize
        let image: UIImage
    }

    @MainActor final class MountedView {
        private let window: UIWindow
        private let parent: UIViewController
        let controller: UIViewController
        private let defaults: UserDefaults
        private let defaultsName: String
        private(set) var maximumPresentationDepth = 0

        fileprivate init(window: UIWindow, parent: UIViewController, controller: UIViewController,
                         defaults: UserDefaults, defaultsName: String) {
            self.window = window
            self.parent = parent
            self.controller = controller
            self.defaults = defaults
            self.defaultsName = defaultsName
        }

        var presentedControllers: [UIViewController] {
            var controllers: [UIViewController] = []
            var presented = controller.presentedViewController ?? parent.presentedViewController
            while let current = presented {
                controllers.append(current)
                presented = current.presentedViewController
            }
            return controllers
        }

        func wait(until description: String, _ condition: () -> Bool) async throws {
            let clock = ContinuousClock()
            let deadline = clock.now.advanced(by: .seconds(5))
            while clock.now < deadline {
                controller.view.setNeedsLayout()
                controller.view.layoutIfNeeded()
                maximumPresentationDepth = max(maximumPresentationDepth, presentedControllers.count)
                if condition() { return }
                try await Task.sleep(for: .milliseconds(10))
            }
            XCTFail("Timed out waiting for \(description)")
            throw PresentationTimeout()
        }

        func settle() async {
            for _ in 0..<10 {
                controller.view.setNeedsLayout()
                controller.view.layoutIfNeeded()
                maximumPresentationDepth = max(maximumPresentationDepth, presentedControllers.count)
                await Task.yield()
            }
        }

        func hasVisibleView(identifier: String) -> Bool {
            visibleViewFrame(identifier: identifier) != nil
        }

        func containsView<ViewType: UIView>(ofType type: ViewType.Type) -> Bool {
            func contains(in view: UIView) -> Bool {
                view is ViewType || view.subviews.contains(where: contains)
            }
            return contains(in: controller.view)
        }

        func visibleControls(in region: CGRect) -> [UIControl] {
            func controls(in view: UIView) -> [UIControl] {
                guard !view.isHidden, view.alpha > 0.01, view.window != nil else { return [] }
                let frame = view.convert(view.bounds, to: controller.view)
                var matches: [UIControl] = []
                if let control = view as? UIControl,
                   !frame.isEmpty,
                   region.contains(CGPoint(x: frame.midX, y: frame.midY)) {
                    matches.append(control)
                }
                return matches + view.subviews.flatMap(controls)
            }
            return controls(in: controller.view)
        }

        func tabBarItemTitles() -> [String] {
            func tabBar(in view: UIView) -> UITabBar? {
                if let bar = view as? UITabBar { return bar }
                return view.subviews.lazy.compactMap(tabBar).first
            }
            return tabBar(in: controller.view)?.items?.compactMap(\.title) ?? []
        }

        func visibleViewFrame(identifier: String) -> CGRect? {
            func frame(in view: UIView) -> CGRect? {
                guard !view.isHidden, view.alpha > 0.01, view.window != nil else { return nil }
                let visibleFrame = view.convert(view.bounds, to: controller.view)
                if view.accessibilityIdentifier == identifier,
                   !visibleFrame.isEmpty, visibleFrame.intersects(controller.view.bounds) {
                    return view.convert(view.bounds, to: nil)
                }
                return view.subviews.lazy.compactMap(frame).first
            }
            return frame(in: controller.view)
        }

        func accessibilityElement(identifier: String? = nil, label: String? = nil) -> AccessibilityElement? {
            accessibilityCandidates().map(\.element).first { element in
                (identifier == nil || element.identifier == identifier)
                    && (label == nil || element.label == label)
            }
        }

        func hasAccessibilityElement(label: String) -> Bool {
            accessibilityElement(label: label) != nil
        }

        func hasAccessibilityTrait(_ trait: UIAccessibilityTraits) -> Bool {
            accessibilitySnapshot().contains { $0.traits.contains(trait) }
        }

        func activateAccessibilityElement(label: String) -> Bool {
            guard let candidate = accessibilityCandidates().first(where: { $0.element.label == label }) else {
                return false
            }
            return candidate.object.accessibilityActivate()
        }

        func selectAllTasks() -> Bool {
            UIApplication.shared.sendAction(
                #selector(UIResponderStandardEditActions.selectAll(_:)),
                to: nil,
                from: nil,
                for: nil
            )
        }

        func accessibilitySnapshot() -> [AccessibilityElement] {
            accessibilityCandidates().map(\.element)
        }

        func accessibilityLabels() -> [String] {
            accessibilitySnapshot().compactMap(\.label)
        }

        private func accessibilityCandidates() -> [AccessibilityCandidate] {
            controller.view.setNeedsLayout()
            controller.view.layoutIfNeeded()
            var visited: Set<ObjectIdentifier> = []
            return accessibilityElements(in: controller.view, visited: &visited)
        }

        private func accessibilityElements(
            in object: NSObject,
            visited: inout Set<ObjectIdentifier>
        ) -> [AccessibilityCandidate] {
            guard visited.insert(ObjectIdentifier(object)).inserted else { return [] }
            guard !object.accessibilityElementsHidden else { return [] }
            if let view = object as? UIView {
                guard !view.isHidden, view.alpha > 0.01, view.window != nil else { return [] }
            }

            var result: [AccessibilityCandidate] = []
            let identifier: String?
            if let view = object as? UIView {
                identifier = view.accessibilityIdentifier
            } else if let element = object as? UIAccessibilityElement {
                identifier = element.accessibilityIdentifier
            } else {
                identifier = nil
            }
            let frame: CGRect
            if let view = object as? UIView {
                frame = view.convert(view.bounds, to: nil)
            } else {
                frame = object.accessibilityFrame
            }
            let visibleBounds = controller.view.convert(controller.view.bounds, to: nil)
            if object.isAccessibilityElement, !frame.isEmpty, frame.intersects(visibleBounds) {
                result.append(AccessibilityCandidate(
                    element: AccessibilityElement(
                        identifier: identifier,
                        label: object.accessibilityLabel,
                        frame: frame,
                        traits: object.accessibilityTraits
                    ),
                    object: object
                ))
            }

            var children: [NSObject] = []
            let count = object.accessibilityElementCount()
            if count != NSNotFound, count > 0 {
                for index in 0..<count {
                    if let child = object.accessibilityElement(at: index) as? NSObject {
                        children.append(child)
                    }
                }
            }
            if children.isEmpty {
                for child in object.accessibilityElements ?? [] {
                    if let child = child as? NSObject { children.append(child) }
                }
            }
            if children.isEmpty {
                for child in object.automationElements ?? [] {
                    if let child = child as? NSObject { children.append(child) }
                }
            }
            if children.isEmpty, let view = object as? UIView {
                children = view.subviews
            }
            for child in children {
                result.append(contentsOf: accessibilityElements(in: child, visited: &visited))
            }
            return result
        }

        func updateViewport(size: CGSize, horizontalSizeClass: UIUserInterfaceSizeClass) {
            controller.traitOverrides.horizontalSizeClass = horizontalSizeClass
            controller.view.frame = CGRect(origin: .zero, size: size)
            controller.view.setNeedsUpdateConstraints()
            controller.view.setNeedsLayout()
            controller.view.layoutIfNeeded()
        }

        func renderedImage() -> UIImage {
            controller.view.setNeedsLayout()
            controller.view.layoutIfNeeded()
            let bounds = controller.view.bounds
            let format = UIGraphicsImageRendererFormat()
            format.scale = 1
            format.opaque = true
            return UIGraphicsImageRenderer(bounds: bounds, format: format).image { _ in
                XCTAssertTrue(controller.view.drawHierarchy(in: bounds, afterScreenUpdates: true))
            }
        }

        func unmount() {
            controller.dismiss(animated: false)
            controller.beginAppearanceTransition(false, animated: false)
            controller.willMove(toParent: nil)
            controller.view.removeFromSuperview()
            controller.removeFromParent()
            controller.endAppearanceTransition()
            window.isHidden = true
            window.rootViewController = nil
            defaults.removePersistentDomain(forName: defaultsName)
        }
    }

    struct AccessibilityElement {
        let identifier: String?
        let label: String?
        let frame: CGRect
        let traits: UIAccessibilityTraits
    }

    private struct AccessibilityCandidate {
        let element: AccessibilityElement
        let object: NSObject
    }

    static func render<V: View>(_ content: V, width: CGFloat = 320,
                                size: DynamicTypeSize = .large,
                                scheme: ColorScheme = .light,
                                contrast: ColorSchemeContrast = .standard,
                                locale: String = "en",
                                contentScale: Double = 1) throws -> Render {
        let name = "momentum-view-test-\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        defaults.set(contentScale, forKey: MobileAppearance.contentScaleKey)
        let root = content
            .environment(\.dynamicTypeSize, size)
            .environment(\.colorScheme, scheme)
            .environment(\.locale, Locale(identifier: locale))
            .transaction { $0.animation = nil; $0.disablesAnimations = true }
            .momentumAccent()
            .defaultAppStorage(defaults)
        let controller = UIHostingController(rootView: root)
        controller.overrideUserInterfaceStyle = scheme == .dark ? .dark : .light
        let parent = try parentController()
        let traits = UITraitCollection(traitsFrom: [
            UITraitCollection(userInterfaceStyle: scheme == .dark ? .dark : .light),
            UITraitCollection(accessibilityContrast: contrast == .increased ? .high : .normal),
        ])
        // This is a component viewport, not a screen/toolbar safe-area test.
        controller.safeAreaRegions = []
        parent.addChild(controller)
        parent.setOverrideTraitCollection(traits, forChild: controller)
        controller.beginAppearanceTransition(true, animated: false)
        parent.view.addSubview(controller.view)
        controller.didMove(toParent: parent)
        controller.endAppearanceTransition()
        defer {
            parent.setOverrideTraitCollection(nil, forChild: controller)
            controller.beginAppearanceTransition(false, animated: false)
            controller.willMove(toParent: nil)
            controller.view.removeFromSuperview()
            controller.removeFromParent()
            controller.endAppearanceTransition()
        }
        let fitting = controller.sizeThatFits(in: CGSize(width: width, height: 10_000))
        guard fitting.width.isFinite, fitting.height.isFinite,
              fitting.width > 0, fitting.height > 0, fitting.height < 10_000 else {
            throw InvalidLayout(size: fitting)
        }
        let bounds = CGRect(x: 0, y: 0, width: width, height: ceil(fitting.height))
        controller.view.frame = bounds
        controller.view.backgroundColor = scheme == .dark ? .black : .white
        controller.view.setNeedsLayout()
        controller.view.layoutIfNeeded()
        let format = UIGraphicsImageRendererFormat()
        format.scale = 1
        format.opaque = true
        var drew = false
        let image = UIGraphicsImageRenderer(bounds: bounds, format: format).image { _ in
            drew = controller.view.drawHierarchy(in: bounds, afterScreenUpdates: true)
        }
        XCTAssertTrue(drew, "UIKit must render the view, not silently return a blank snapshot")
        return Render(size: fitting, image: image)
    }

    static func mount<V: View>(_ content: V, defaults: UserDefaults,
                               defaultsName: String, size: CGSize = CGSize(width: 390, height: 844),
                               horizontalSizeClass: UIUserInterfaceSizeClass? = nil,
                               locale: String = "en",
                               dynamicTypeSize: DynamicTypeSize = .large,
                               animationsEnabled: Bool = false) throws -> MountedView {
        let root = content
            .environment(\.accessibilityEnabled, true)
            .environment(\.locale, Locale(identifier: locale))
            .environment(\.dynamicTypeSize, dynamicTypeSize)
            .transaction {
                if !animationsEnabled {
                    $0.animation = nil
                    $0.disablesAnimations = true
                }
            }
            .momentumAccent()
            .defaultAppStorage(defaults)
        let controller = UIHostingController(rootView: root)
        if let horizontalSizeClass {
            controller.traitOverrides.horizontalSizeClass = horizontalSizeClass
        }
        let scene = try XCTUnwrap(UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }.first)
        let window = UIWindow(windowScene: scene)
        let parent = UIViewController()
        window.rootViewController = parent
        window.makeKeyAndVisible()
        controller.view.frame = CGRect(origin: .zero, size: size)
        parent.addChild(controller)
        controller.beginAppearanceTransition(true, animated: false)
        parent.view.addSubview(controller.view)
        controller.didMove(toParent: parent)
        controller.endAppearanceTransition()
        controller.view.setNeedsLayout()
        controller.view.layoutIfNeeded()
        return MountedView(window: window, parent: parent, controller: controller,
                           defaults: defaults, defaultsName: defaultsName)
    }

    private struct InvalidLayout: Error { let size: CGSize }
    private struct PresentationTimeout: Error {}

}
