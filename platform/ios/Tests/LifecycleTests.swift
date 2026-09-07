import XCTest

final class LifecycleTests: XCTestCase {
    func testBackgroundResume() {
        continueAfterFailure = false
        let app = XCUIApplication(bundleIdentifier: "net.pixbs.claimlands")
        // smoke.sh starts the application with native stdout/stderr capture.
        // Attach to that process so renderer markers survive the lifecycle test.
        XCTAssertTrue(app.state == .runningForeground || app.state == .runningBackground
            || app.state == .runningBackgroundSuspended)
        app.activate()
        XCTAssertTrue(app.wait(for: .runningForeground, timeout: 20))
        XCTAssertTrue(app.windows.firstMatch.waitForExistence(timeout: 20))
        attachScreenshot("initial")

        let window = app.windows.firstMatch
        let start = window.coordinate(withNormalizedOffset: CGVector(dx: 0.50, dy: 0.55))
        let end = window.coordinate(withNormalizedOffset: CGVector(dx: 0.70, dy: 0.65))
        start.press(forDuration: 0.1, thenDragTo: end)

        XCUIDevice.shared.press(.home)
        let background = NSPredicate { _, _ in
            app.state == .runningBackground || app.state == .runningBackgroundSuspended
        }
        expectation(for: background, evaluatedWith: nil)
        waitForExpectations(timeout: 20)
        app.activate()
        XCTAssertTrue(app.wait(for: .runningForeground, timeout: 20))
        XCTAssertTrue(app.windows.firstMatch.waitForExistence(timeout: 20))
        attachScreenshot("resumed")
    }

    private func attachScreenshot(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
