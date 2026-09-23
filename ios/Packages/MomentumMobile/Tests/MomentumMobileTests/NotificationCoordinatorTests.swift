// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore
import Synchronization
import Testing
@testable import MomentumMobile

private actor NotificationGate {
    private var entered = false
    private var released = false
    private var arrival: [CheckedContinuation<Void, Never>] = []
    private var release: CheckedContinuation<Void, Never>?

    func pause() async {
        if released { return }
        await withCheckedContinuation { continuation in
            release = continuation
            entered = true
            arrival.forEach { $0.resume() }
            arrival.removeAll()
        }
    }
    func waitForArrival() async {
        if entered { return }
        await withCheckedContinuation { arrival.append($0) }
    }
    func resume() { released = true; release?.resume(); release = nil }
}

private final class NotificationTestClock: Sendable {
    private let value: Mutex<UInt64>
    init(_ now: UInt64) { value = Mutex(now) }
    func now() -> UInt64 { value.withLock { $0 } }
    func set(_ now: UInt64) { value.withLock { $0 = now } }
}

private actor NotificationEvents {
    var values: [String] = []
    func append(_ value: String) { values.append(value) }
}

private struct PrivateNotificationError: Error {
    let sensitiveDescription = "Private task title and revision must not escape"
}

private actor FakeSchedulingCore: NotificationSchedulingCore {
    var plan: NotificationPlan
    var accepted: [NotificationRequest] = []
    var reconciledPending: [[NotificationObservation]] = []
    var reconciledDelivered: [[NotificationObservation]] = []
    var reconciliationTimes: [UInt64] = []
    var acceptanceTimes: [UInt64] = []
    var rejects = false
    var failsAcceptance = false
    var failsPlanning = false
    let gate: NotificationGate?
    let events: NotificationEvents

    init(plan: NotificationPlan, events: NotificationEvents, gate: NotificationGate? = nil) {
        self.plan = plan; self.events = events; self.gate = gate
    }
    func notificationPlan(nowMs: UInt64) async throws -> NotificationPlan {
        if failsPlanning { throw PrivateNotificationError() }
        return plan
    }
    func reconcileNotifications(nowMs: UInt64, pending: [NotificationObservation], delivered: [NotificationObservation]) async throws -> NotificationPlan {
        await events.append("reconcile")
        if failsPlanning { throw PrivateNotificationError() }
        reconciledPending.append(pending); reconciledDelivered.append(delivered)
        reconciliationTimes.append(nowMs)
        return plan
    }
    func acceptNotification(request: NotificationRequest, nowMs: UInt64) async throws -> NotificationAcceptance {
        await events.append("ack")
        acceptanceTimes.append(nowMs)
        if let gate { await gate.pause() }
        if failsAcceptance { throw PrivateNotificationError() }
        if rejects { return NotificationAcceptance(accepted: false, cancellation: observation(request)) }
        accepted.append(request)
        plan.accepted.append(observation(request))
        return NotificationAcceptance(accepted: true, cancellation: nil)
    }
    func setRejection(_ value: Bool) { rejects = value }
    func setFailure(_ value: Bool) { failsAcceptance = value }
    func failPlanning() { failsPlanning = true }
    func setPlan(_ value: NotificationPlan) { plan = value }
}

private actor FakeSchedulingCenter: NotificationSchedulingCenter {
    var permission: NotificationAuthorization = .authorized
    var pending: [NotificationCenterRecord] = []
    var delivered: [NotificationCenterRecord] = []
    var additions: [NotificationSubmission] = []
    var removedPending: [[String]] = []
    var removedDelivered: [[String]] = []
    var failingAdds: Set<String> = []
    var delayedRemoval = false
    var failsAuthorization = false
    var failsSnapshots = false
    var failsReadback = false
    var transitionOnPendingRead = false
    let gate: NotificationGate?
    let events: NotificationEvents

    init(events: NotificationEvents, gate: NotificationGate? = nil) { self.events = events; self.gate = gate }
    func authorization() async throws -> NotificationAuthorization {
        if failsAuthorization { throw PrivateNotificationError() }
        return permission
    }
    func pendingRequests() async throws -> [NotificationCenterRecord] {
        await events.append("pending")
        if failsSnapshots || (failsReadback && !removedPending.isEmpty) { throw PrivateNotificationError() }
        let result = pending
        if transitionOnPendingRead { delivered += pending; pending = []; transitionOnPendingRead = false }
        return result
    }
    func deliveredRequests() async throws -> [NotificationCenterRecord] {
        await events.append("delivered")
        return delivered
    }
    func add(_ request: NotificationSubmission) async throws {
        await events.append("add")
        if let gate { await gate.pause() }
        if failingAdds.contains(request.identifier) { throw PrivateNotificationError() }
        additions.append(request)
        pending.append(NotificationCenterRecord(identifier: request.identifier, metadata: request.metadata))
    }
    func removePending(identifiers: [String]) async {
        removedPending.append(identifiers)
        if !delayedRemoval { pending.removeAll { identifiers.contains($0.identifier) } }
    }
    func removeDelivered(identifiers: [String]) async {
        removedDelivered.append(identifiers)
        if !delayedRemoval { delivered.removeAll { identifiers.contains($0.identifier) } }
    }
    func setPermission(_ value: NotificationAuthorization) { permission = value }
    func setRecords(pending: [NotificationCenterRecord], delivered: [NotificationCenterRecord] = []) {
        self.pending = pending; self.delivered = delivered
    }
    func failAdd(_ identifier: String) { failingAdds.insert(identifier) }
    func delayRemoval() { delayedRemoval = true }
    func failAuthorization() { failsAuthorization = true }
    func failSnapshots() { failsSnapshots = true }
    func failReadback() { failsReadback = true }
    func transitionOnRead() { transitionOnPendingRead = true }
}

private func request(_ id: String = "task", revision: String = "private-revision", fire: UInt64 = 2_000) -> NotificationRequest {
    NotificationRequest(id: id, sourceRevision: revision, scheduledAtMs: fire, fireAtMs: fire,
                        content: .reminder(taskId: id, title: "Private task", dueAtMs: fire))
}
private func observation(_ request: NotificationRequest) -> NotificationObservation {
    NotificationObservation(id: request.id, sourceRevision: request.sourceRevision)
}
private func record(_ request: NotificationRequest) -> NotificationCenterRecord {
    let value = observation(request)
    return NotificationCenterRecord(identifier: NotificationIdentity.identifier(for: value), metadata: NotificationIdentity.metadata(for: value))
}
private func plan(_ requests: [NotificationRequest], accepted: [NotificationObservation] = [], cancellations: [NotificationObservation] = []) -> NotificationPlan {
    NotificationPlan(accepted: accepted, cancellations: cancellations, horizonEndMs: 604_800_000,
                     requests: requests, overflowReminders: 3, overflowSummaries: 2, projectedOccurrences: [])
}
private func coordinator(_ core: FakeSchedulingCore, _ center: FakeSchedulingCenter) -> NotificationCoordinator {
    NotificationCoordinator(core: core, center: center, nowMs: { 1_000 }, formatContent: { _ in
        NotificationText(title: "Localized title", body: "Localized body")
    })
}

@Suite struct NotificationCoordinatorTests {
    @Test func desiredMinusAcceptedUsesSequentialSnapshotsAndAddBeforeAck() async {
        let events = NotificationEvents()
        let old = request("old"), new = request("new")
        let core = FakeSchedulingCore(plan: plan([old, new], accepted: [observation(old)]), events: events)
        let center = FakeSchedulingCenter(events: events)
        await center.setRecords(pending: [record(old)])
        let status = await coordinator(core, center).refresh()
        #expect(await events.values == ["pending", "delivered", "reconcile", "add", "ack"])
        #expect(await core.accepted == [new])
        #expect(await center.additions.count == 1)
        #expect(await center.additions.first?.title == "Localized title")
        #expect(status.acceptedCount == 2)
        #expect(status.horizonEndMs == 604_800_000)
        #expect(status.overflowReminders == 3 && status.overflowSummaries == 2)
        #expect(status.issues.isEmpty)
    }

    @Test(arguments: [NotificationAuthorization.denied, .notDetermined, .unavailable])
    func unauthorizedOnlyReportsPlan(_ permission: NotificationAuthorization) async {
        let events = NotificationEvents()
        let core = FakeSchedulingCore(plan: plan([request()]), events: events)
        let center = FakeSchedulingCenter(events: events)
        await center.setPermission(permission)
        let status = await coordinator(core, center).refresh()
        #expect(status.authorization == permission)
        #expect(status.horizonEndMs == 604_800_000)
        #expect(await center.additions.isEmpty)
        #expect(await core.accepted.isEmpty)
        #expect(await core.reconciledPending.isEmpty)
    }

    @Test func provisionalAuthorizationAndPositiveCatchUpInterval() async {
        let events = NotificationEvents()
        let item = request(fire: 500)
        let core = FakeSchedulingCore(plan: plan([item]), events: events)
        let center = FakeSchedulingCenter(events: events)
        await center.setPermission(.provisional)
        let status = await coordinator(core, center).refresh()
        #expect(status.authorization == .provisional)
        #expect(await center.additions.first?.timeInterval == 1)
        #expect(await core.accepted == [item], "Acknowledge the original core timestamps")
    }

    @Test func disablingSummaryWhileDeniedStillCancelsItsAcceptedFutureRequest() async {
        let events = NotificationEvents()
        let old = request("summary"), untouched = request("still-valid")
        let core = FakeSchedulingCore(plan: plan([untouched], cancellations: [observation(old)]), events: events)
        let center = FakeSchedulingCenter(events: events)
        await center.setRecords(pending: [record(old), record(untouched)])
        await center.setPermission(.denied)
        let status = await coordinator(core, center).refresh()
        #expect(await center.pending == [record(untouched)])
        #expect(await center.additions.isEmpty)
        #expect(await core.accepted.isEmpty)
        #expect(await core.reconciledPending.isEmpty)
        #expect(status.authorization == .denied)
        #expect(status.issues.isEmpty)
    }

    @Test func partialAddFailureDoesNotAcknowledgeOrLeakPrivateError() async {
        let events = NotificationEvents()
        let first = request("first"), second = request("second")
        let core = FakeSchedulingCore(plan: plan([first, second]), events: events)
        let center = FakeSchedulingCenter(events: events)
        await center.failAdd(record(first).identifier)
        let status = await coordinator(core, center).refresh()
        #expect(await core.accepted == [second])
        #expect(status.acceptedCount == 1)
        #expect(status.issues == [.schedulingFailed])
        #expect(!String(describing: status).contains("Private"))
    }

    @Test(arguments: [false, true]) func rejectionOrPersistenceFailureCancelsExactSuccessfulAdd(failure: Bool) async {
        let events = NotificationEvents()
        let item = request()
        let core = FakeSchedulingCore(plan: plan([item]), events: events)
        await core.setFailure(failure); await core.setRejection(!failure)
        let center = FakeSchedulingCenter(events: events)
        let status = await coordinator(core, center).refresh()
        #expect(await center.pending.isEmpty)
        #expect(await center.removedPending == [[record(item).identifier]])
        #expect(await center.removedDelivered == [[record(item).identifier]])
        #expect(status.issues == [failure ? .acceptanceFailed : .sourceChanged])
        #expect(status.unconfirmedCancellationCount == 0)
    }

    @Test func malformedOwnedRecordsAreRemovedAndUnrelatedRecordsAreIgnored() async {
        let events = NotificationEvents()
        let item = request()
        let malformed = NotificationCenterRecord(identifier: NotificationIdentity.prefix + "malformed", metadata: [:])
        let unrelated = NotificationCenterRecord(identifier: "unrelated", metadata: [:])
        let core = FakeSchedulingCore(plan: plan([], accepted: []), events: events)
        let center = FakeSchedulingCenter(events: events)
        await center.setRecords(pending: [malformed, unrelated], delivered: [record(item)])
        _ = await coordinator(core, center).refresh()
        #expect(await core.reconciledPending == [[]])
        #expect(await core.reconciledDelivered == [[observation(item)]])
        #expect(await center.removedPending == [[malformed.identifier]])
        #expect(await center.pending == [unrelated])
    }

    @Test func unconfirmedOldRevisionCancellationStopsBeforeReplacement() async {
        let events = NotificationEvents()
        let old = request(revision: "old"), new = request(revision: "new")
        let core = FakeSchedulingCore(plan: plan([new], cancellations: [observation(old)]), events: events)
        let center = FakeSchedulingCenter(events: events)
        await center.setRecords(pending: [record(old)])
        await center.delayRemoval()
        let status = await coordinator(core, center).refresh()
        #expect(status.issues == [.cancellationUnconfirmed])
        #expect(status.unconfirmedCancellationCount == 1)
        #expect(await center.additions.isEmpty)
        #expect(await center.removedPending == [[record(old).identifier]])
        #expect(record(old).identifier != record(new).identifier)
    }

    @Test func pendingToDeliveredTransitionIsSeenByReconciliation() async {
        let events = NotificationEvents()
        let item = request()
        let core = FakeSchedulingCore(plan: plan([]), events: events)
        let center = FakeSchedulingCenter(events: events)
        await center.setRecords(pending: [record(item)])
        await center.transitionOnRead()
        _ = await coordinator(core, center).refresh()
        #expect(await core.reconciledPending == [[observation(item)]])
        #expect(await core.reconciledDelivered == [[observation(item)]])
    }

    @Test func editsDuringAddAreCoalescedIntoOneMorePass() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let core = FakeSchedulingCore(plan: plan([request()]), events: events)
        let center = FakeSchedulingCenter(events: events, gate: gate)
        let scheduler = coordinator(core, center)
        let first = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        await scheduler.requestRefresh()
        await scheduler.requestRefresh()
        await gate.resume()
        let status = await first.value
        #expect(await core.reconciledPending.count == 2)
        #expect(await center.additions.count == 1)
        #expect(status.issues.isEmpty)
    }

    @Test func cancellingWaiterDoesNotCancelSharedDrain() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let core = FakeSchedulingCore(plan: plan([request()]), events: events)
        let center = FakeSchedulingCenter(events: events, gate: gate)
        let scheduler = coordinator(core, center)
        let waiter = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        waiter.cancel()
        await gate.resume()
        _ = await waiter.value
        #expect(await core.accepted.count == 1)
        #expect(await center.removedPending.isEmpty)
    }

    @Test func explicitCancellationAfterAddBeforeAckCleansUp() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let core = FakeSchedulingCore(plan: plan([request()]), events: events)
        let center = FakeSchedulingCenter(events: events, gate: gate)
        let scheduler = coordinator(core, center)
        let waiter = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        await scheduler.cancelPendingRefresh()
        await gate.resume()
        let status = await waiter.value
        #expect(await core.accepted.isEmpty)
        #expect(await center.pending.isEmpty)
        #expect(status.wasCancelled)
    }

    @Test func cancellationDuringAcknowledgementPreservesAcceptedCatchUp() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let core = FakeSchedulingCore(plan: plan([request(fire: 500)]), events: events, gate: gate)
        let center = FakeSchedulingCenter(events: events)
        let scheduler = coordinator(core, center)
        let waiter = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        await scheduler.cancelPendingRefresh()
        await gate.resume()
        let status = await waiter.value
        #expect(await core.accepted.count == 1)
        #expect(await center.pending.count == 1)
        #expect(await center.removedPending.isEmpty)
        #expect(status.acceptedCount == 1)
        #expect(status.wasCancelled)
    }

    @Test func changedSourceDuringAddCancelsOldRevisionWithoutRemovingNewerOne() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let old = request(revision: "old"), new = request(revision: "new")
        let core = FakeSchedulingCore(plan: plan([old]), events: events)
        let center = FakeSchedulingCenter(events: events, gate: gate)
        let scheduler = coordinator(core, center)
        let waiter = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        await core.setRejection(true)
        await center.setRecords(pending: [record(new)])
        await gate.resume()
        let status = await waiter.value
        #expect(status.issues == [.sourceChanged])
        #expect(await center.pending == [record(new)])
        #expect(await center.removedPending == [[record(old).identifier]])
    }

    @Test func laterRefreshSurvivesCancellationAndForegroundCanReauthorize() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let core = FakeSchedulingCore(plan: plan([request()]), events: events)
        let center = FakeSchedulingCenter(events: events, gate: gate)
        let scheduler = coordinator(core, center)
        let waiter = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        await scheduler.cancelPendingRefresh()
        await center.setPermission(.denied)
        await scheduler.requestRefresh()
        await gate.resume()
        let cancelledThenRefreshed = await waiter.value
        #expect(cancelledThenRefreshed.authorization == .denied)
        #expect(!cancelledThenRefreshed.wasCancelled)
        #expect(await core.accepted.isEmpty)
        await center.setPermission(.authorized)
        let authorized = await scheduler.refresh()
        #expect(authorized.acceptedCount == 1)
        #expect(authorized.issues.isEmpty)
    }

    @Test func acceptanceReadsFreshClockAfterAsyncAdd() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let clock = NotificationTestClock(1_000)
        let core = FakeSchedulingCore(plan: plan([request()]), events: events)
        let center = FakeSchedulingCenter(events: events, gate: gate)
        let scheduler = NotificationCoordinator(core: core, center: center, nowMs: { clock.now() },
            formatContent: { _ in NotificationText(title: "Reminder", body: "") })
        let waiter = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        clock.set(3_000)
        await gate.resume()
        _ = await waiter.value
        #expect(await core.reconciliationTimes == [1_000])
        #expect(await core.acceptanceTimes == [3_000])
    }

    @Test func realWorkerRecoversCrashWindowReplenishesMissingFutureAndSuppressesDismissedElapsed() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("momentum-notifications-\(UUID())")
        defer { try? FileManager.default.removeItem(at: directory) }
        let worker = await EngineWorker.open(directory: directory)
        let draft = TaskDraft(title: "Isolated reminder", projectId: "", dueDay: dayOffset(day: today(), days: 1),
            time: ClockTime(hour: 12, minute: 0), reminderMinutesBefore: 5, estimateMs: 0,
            notes: "", tagIds: [], newTags: [])
        #expect(await worker.createTask(draft, view: .today).changed)
        let clock = NotificationTestClock(MomentumCore.nowMs())
        let initial = try await worker.notificationPlan(nowMs: clock.now())
        let item = try #require(initial.requests.first)
        let events = NotificationEvents()
        let center = FakeSchedulingCenter(events: events)
        // Represents a process dying after successful OS add, before core acceptance.
        await center.setRecords(pending: [record(item)])
        let scheduler = NotificationCoordinator(core: worker, center: center, nowMs: { clock.now() },
            formatContent: { _ in NotificationText(title: "Reminder", body: "") })
        let recovered = await scheduler.refresh()
        #expect(recovered.acceptedCount == 1)
        #expect(await center.additions.isEmpty)
        // The OS loses a future request: reconciliation must permit replenishment.
        await center.setRecords(pending: [])
        let replenished = await scheduler.refresh()
        #expect(replenished.acceptedCount == 1)
        #expect(await center.additions.count == 1)
        // Delivered and dismissed after expiry, then restart: never duplicate it.
        clock.set(item.fireAtMs + 1)
        await center.setRecords(pending: [])
        let reopened = await EngineWorker.open(directory: directory)
        let restarted = NotificationCoordinator(core: reopened, center: center, nowMs: { clock.now() },
            formatContent: { _ in NotificationText(title: "Reminder", body: "") })
        let elapsed = await restarted.refresh()
        #expect(elapsed.acceptedCount == 0)
        #expect(await center.additions.count == 1)
        #expect(elapsed.issues.isEmpty)
    }


    @Test(arguments: [NotificationSchedulingIssue.authorizationUnavailable, .observationsUnavailable, .planUnavailable])
    func incompleteInputsNeverScheduleOrAcknowledge(_ failure: NotificationSchedulingIssue) async {
        let events = NotificationEvents()
        let core = FakeSchedulingCore(plan: plan([request()]), events: events)
        let center = FakeSchedulingCenter(events: events)
        switch failure {
        case .authorizationUnavailable: await center.failAuthorization()
        case .observationsUnavailable: await center.failSnapshots()
        default: await core.failPlanning()
        }
        let status = await coordinator(core, center).refresh()
        #expect(status.issues == [failure])
        #expect(await center.additions.isEmpty)
        #expect(await core.accepted.isEmpty)
        #expect(await center.removedPending.isEmpty)
    }

    @Test func failedCleanupReadbackDoesNotClaimCancellationOrContinueAdding() async {
        let events = NotificationEvents()
        let core = FakeSchedulingCore(plan: plan([request("first"), request("second")]), events: events)
        await core.setRejection(true)
        let center = FakeSchedulingCenter(events: events)
        await center.failReadback()
        let status = await coordinator(core, center).refresh()
        #expect(status.issues == [.sourceChanged, .observationsUnavailable, .cancellationUnconfirmed])
        #expect(status.unconfirmedCancellationCount == 1)
        #expect(await center.additions.count == 1)
    }

    @Test func alreadyCancelledCallerDoesNotStartRefresh() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let core = FakeSchedulingCore(plan: plan([request()]), events: events)
        let center = FakeSchedulingCenter(events: events)
        let scheduler = coordinator(core, center)
        let waiter = Task {
            await gate.pause()
            return await scheduler.refresh()
        }
        await gate.waitForArrival()
        waiter.cancel()
        await gate.resume()
        _ = await waiter.value
        #expect(await events.values.isEmpty)
        #expect(await center.additions.isEmpty)
    }


    @Test(arguments: [NotificationAuthorization.denied, .notDetermined, .unavailable])
    func authorizationRevokedDuringFailedAddStopsRemainingRequests(_ permission: NotificationAuthorization) async {
        let events = NotificationEvents(), gate = NotificationGate()
        let first = request("first"), second = request("second")
        let core = FakeSchedulingCore(plan: plan([first, second]), events: events)
        let center = FakeSchedulingCenter(events: events, gate: gate)
        await center.failAdd(record(first).identifier)
        let scheduler = coordinator(core, center)
        let waiter = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        await center.setPermission(permission)
        await gate.resume()
        let status = await waiter.value
        #expect(status.authorization == permission)
        #expect(status.issues == [.schedulingFailed])
        #expect(await events.values.filter { $0 == "add" }.count == 1)
        #expect(await center.additions.isEmpty)
        #expect(await core.accepted.isEmpty)
        #expect(await core.acceptanceTimes.isEmpty)
    }

    @Test func authorizationLookupFailureAfterFailedAddStopsAndClearsStaleStatus() async {
        let events = NotificationEvents(), gate = NotificationGate()
        let first = request("first"), second = request("second")
        let core = FakeSchedulingCore(plan: plan([first, second]), events: events)
        let center = FakeSchedulingCenter(events: events, gate: gate)
        await center.failAdd(record(first).identifier)
        let scheduler = coordinator(core, center)
        let waiter = Task { await scheduler.refresh() }
        await gate.waitForArrival()
        await center.failAuthorization()
        await gate.resume()
        let status = await waiter.value
        #expect(status.authorization == .unavailable)
        #expect(status.issues == [.schedulingFailed, .authorizationUnavailable])
        #expect(await events.values.filter { $0 == "add" }.count == 1)
        #expect(await center.additions.isEmpty)
        #expect(await core.accepted.isEmpty)
        #expect(await core.acceptanceTimes.isEmpty)
        #expect(!String(describing: status).contains("Private"))
    }


    @Test func typedRoutesComeFromCoreContentWithoutParsingOpaqueIdentifiers() async {
        let reminder = NotificationRequest(id: "opaque-one", sourceRevision: "private-reminder-revision",
            scheduledAtMs: 2_000, fireAtMs: 2_000,
            content: .reminder(taskId: "core-task-id", title: "Task", dueAtMs: 2_000))
        let summary = NotificationRequest(id: "opaque-two", sourceRevision: "private-summary-revision",
            scheduledAtMs: 3_000, fireAtMs: 3_000,
            content: .summary(day: "2030-01-02", counts: MorningSummary(total: 2, morning: 1, tonight: 0), taskIds: ["a", "b"]))
        let events = NotificationEvents()
        let core = FakeSchedulingCore(plan: plan([reminder, summary]), events: events)
        let center = FakeSchedulingCenter(events: events)
        _ = await coordinator(core, center).refresh()
        #expect(await center.additions.map(\.route) == [.reminder(taskID: "core-task-id"), .summary(day: "2030-01-02")])
        #expect(await core.accepted == [reminder, summary])
    }

}
