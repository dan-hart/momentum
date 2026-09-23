// SPDX-License-Identifier: GPL-3.0-or-later
import MomentumKit
import Testing
@testable import MomentumMobile

@Suite struct EditorValidationTests {
    @Test(arguments: ["", " ", "\n\t"])
    func blankTitleCannotSave(_ title: String) {
        var form = TaskFormModel()
        form.title = title
        #expect(!form.mobileCanSave)
    }

    @Test(arguments: ["", " ", "30m", "1h 30m"])
    func validEstimateCanSave(_ estimate: String) {
        var form = TaskFormModel()
        form.title = "Task"
        form.estimateText = estimate
        #expect(!form.mobileEstimateInvalid)
        #expect(form.mobileCanSave)
    }

    @Test func invalidEstimateBlocksSaveUntilCorrected() {
        var form = TaskFormModel()
        form.title = "Task"
        form.estimateText = "invalid"
        #expect(form.mobileEstimateInvalid)
        #expect(!form.mobileCanSave)
        form.estimateText = "45m"
        #expect(form.mobileCanSave)
    }

    @Test func invalidScheduledTimeBlocksSave() {
        var form = TaskFormModel()
        form.title = "Task"
        form.dueDay = "2026-09-16"
        form.timeText = "25:99"
        #expect(!form.mobileCanSave)
        form.timeText = "09:00"
        #expect(form.mobileCanSave)
    }
}
