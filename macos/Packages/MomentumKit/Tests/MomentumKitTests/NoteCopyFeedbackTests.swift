// SPDX-License-Identifier: GPL-3.0-or-later
import Testing
@testable import MomentumKit

@MainActor @Suite struct NoteCopyFeedbackTests {
    @Test func fullUnicodeNotesAreCopiedAndSuccessFeedbackIsEmittedOnce() {
        var copied: String?
        var feedback = 0
        let presenter = NoteCopyFeedback(write: { copied = $0; return true }, success: { feedback += 1 })
        let notes = "Complete notes 📝\nSecond line — Grüße"
        #expect(presenter.copy(notes))
        #expect(copied == notes)
        #expect(presenter.message != nil)
        #expect(presenter.generation == 1)
        #expect(feedback == 1)
        presenter.dismiss()
        #expect(presenter.message == nil)
    }

    @Test func failedCopyDoesNotReportSuccessOrRequestHaptics() {
        var feedback = 0
        let presenter = NoteCopyFeedback(write: { _ in false }, success: { feedback += 1 })
        #expect(!presenter.copy("Notes"))
        #expect(presenter.message == nil)
        #expect(presenter.generation == 0)
        #expect(feedback == 0)
    }

    @Test func aRepeatedCopyReplacesTheDeadlineAndOldDeliveryCannotDismissIt() async {
        let delay = NoteCopyDelay()
        let presenter = NoteCopyFeedback(write: { _ in true }, success: {}, sleep: { await delay.wait() })
        #expect(presenter.copy("First"))
        await delay.waitForCount(1)
        #expect(presenter.copy("Second"))
        await delay.waitForCount(2)
        await delay.release(0)
        for _ in 0..<10 { await Task.yield() }
        #expect(presenter.message != nil, "an earlier timeout cannot dismiss the second acknowledgement")
        await delay.release(1)
        for _ in 0..<10 where presenter.message != nil { await Task.yield() }
        #expect(presenter.message == nil)
    }
}

private actor NoteCopyDelay {
    private var pending: [CheckedContinuation<Void, Never>?] = []
    private var waiters: [(Int, CheckedContinuation<Void, Never>)] = []
    func wait() async {
        await withCheckedContinuation { continuation in
            pending.append(continuation)
            let ready = waiters.filter { $0.0 <= pending.count }
            waiters.removeAll { $0.0 <= pending.count }
            ready.forEach { $0.1.resume() }
        }
    }
    func waitForCount(_ count: Int) async {
        guard pending.count < count else { return }
        await withCheckedContinuation { waiters.append((count, $0)) }
    }
    func release(_ index: Int) { pending[index]?.resume(); pending[index] = nil }
}
