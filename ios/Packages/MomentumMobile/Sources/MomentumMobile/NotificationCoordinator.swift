// SPDX-License-Identifier: GPL-3.0-or-later
import Foundation
import MomentumCore

/// One event-driven drain owns OS scheduling across suspension points. Core owns
/// eligibility, identities and durable claims; the center owns native API access.
public actor NotificationCoordinator {
    private let core: any NotificationSchedulingCore
    private let center: any NotificationSchedulingCenter
    private let nowMs: @Sendable () -> UInt64
    private let formatContent: @Sendable (NotificationContent) -> NotificationText
    private var running: Task<Void, Never>?
    private var dirty = false
    public private(set) var status = NotificationSchedulingStatus()

    public init(core: any NotificationSchedulingCore, center: any NotificationSchedulingCenter,
                nowMs: @escaping @Sendable () -> UInt64,
                formatContent: @escaping @Sendable (NotificationContent) -> NotificationText) {
        self.core = core
        self.center = center
        self.nowMs = nowMs
        self.formatContent = formatContent
    }

    /// Coalesce edits/foreground events, including events arriving during cancellation.
    public func requestRefresh() {
        dirty = true
        if running == nil { startDrain() }
    }

    /// Cancelling this waiter does not cancel the shared drain or accepted requests.
    public func refresh() async -> NotificationSchedulingStatus {
        guard !Task.isCancelled else { return status }
        requestRefresh()
        return await waitUntilIdle()
    }

    public func waitUntilIdle() async -> NotificationSchedulingStatus {
        while let task = running { await task.value }
        return status
    }

    /// Lifecycle shutdown is explicit, separate from an individual waiter's lifetime.
    /// New refresh events after this call are retained for a new, uncancelled drain.
    public func cancelPendingRefresh() {
        dirty = false
        running?.cancel()
    }

    private func startDrain() {
        status.isRefreshing = true
        running = Task { await self.drain() }
    }

    private func drain() async {
        while dirty && !Task.isCancelled {
            dirty = false
            await reconcileOnce()
        }
        status.wasCancelled = Task.isCancelled
        running = nil
        if dirty { startDrain() }
        status.isRefreshing = running != nil
    }

    private func reconcileOnce() async {
        status = NotificationSchedulingStatus()
        status.isRefreshing = true
        let authorization: NotificationAuthorization
        do { authorization = try await center.authorization() }
        catch { status.issues.insert(.authorizationUnavailable); return }
        status.authorization = authorization
        guard !Task.isCancelled else { return }

        // Denial must not accept/consume a request or prompt. A read-only plan still
        // supplies the horizon/overflow UI without pretending anything was scheduled.
        // Opting out must still cancel exact obsolete requests already owned by us.
        guard authorization.permitsScheduling else {
            do {
                let plan = try await core.notificationPlan(nowMs: nowMs())
                recordPlan(plan)
                if !Task.isCancelled {
                    _ = await cancelAndVerify(Set(plan.cancellations.map { NotificationIdentity.identifier(for: $0) }))
                }
            }
            catch { status.issues.insert(.planUnavailable) }
            return
        }

        let pending: [NotificationCenterRecord]
        let delivered: [NotificationCenterRecord]
        do {
            pending = try await center.pendingRequests()
            delivered = try await center.deliveredRequests()
        } catch { status.issues.insert(.observationsUnavailable); return }
        guard !Task.isCancelled else { return }

        let plan: NotificationPlan
        do {
            plan = try await core.reconcileNotifications(nowMs: nowMs(),
                pending: pending.compactMap(decode), delivered: delivered.compactMap(decode))
        } catch { status.issues.insert(.planUnavailable); return }
        recordPlan(plan)
        guard !Task.isCancelled else { return }

        let malformed = (pending + delivered).filter {
            $0.identifier.hasPrefix(NotificationIdentity.prefix) && decode($0) == nil
        }.map(\.identifier)
        let obsolete = plan.cancellations.map { NotificationIdentity.identifier(for: $0) }
        guard await cancelAndVerify(Set(malformed + obsolete)), !Task.isCancelled else { return }

        let accepted = Set(plan.accepted)
        for request in plan.requests {
            guard !Task.isCancelled else { return }
            let identity = NotificationObservation(id: request.id, sourceRevision: request.sourceRevision)
            guard !accepted.contains(identity) else { continue }
            let identifier = NotificationIdentity.identifier(for: identity)
            let text = formatContent(request.content)
            let now = nowMs()
            let interval = request.fireAtMs > now ? Double(request.fireAtMs - now) / 1_000 : 0
            let route: NotificationRoute = switch request.content {
            case .reminder(let taskID, _, _): .reminder(taskID: taskID)
            case .summary(let day, _, _): .summary(day: day)
            }
            let submission = NotificationSubmission(route: route, identifier: identifier,
                metadata: NotificationIdentity.metadata(for: identity), title: text.title,
                body: text.body, timeInterval: max(1, interval))
            do { try await center.add(submission) }
            catch {
                status.issues.insert(.schedulingFailed)
                // Authorization can change while the OS add is suspended. Refresh
                // it before attempting another request, without exposing OS errors.
                do {
                    let current = try await center.authorization()
                    status.authorization = current
                    guard current.permitsScheduling else { return }
                } catch {
                    status.authorization = .unavailable
                    status.issues.insert(.authorizationUnavailable)
                    return
                }
                continue
            }

            guard !Task.isCancelled else {
                _ = await cancelAndVerify([identifier])
                return
            }
            // Commit point: after this call starts, cancellation cannot remove an
            // accepted catch-up request. Its core identity may already be consumed.
            do {
                let result = try await core.acceptNotification(request: request, nowMs: nowMs())
                if result.accepted {
                    status.acceptedCount += 1
                } else {
                    status.issues.insert(.sourceChanged)
                    guard await cancelAndVerify([identifier]) else { return }
                }
            } catch {
                status.issues.insert(.acceptanceFailed)
                guard await cancelAndVerify([identifier]) else { return }
            }
        }
    }

    private func decode(_ record: NotificationCenterRecord) -> NotificationObservation? {
        NotificationIdentity.decode(identifier: record.identifier, metadata: record.metadata)
    }

    private func recordPlan(_ plan: NotificationPlan) {
        status.horizonEndMs = plan.horizonEndMs
        status.overflowReminders = plan.overflowReminders
        status.overflowSummaries = plan.overflowSummaries
        status.acceptedCount = Set(plan.accepted).count
    }

    /// SDK removal is asynchronous without a completion result. A bounded readback
    /// verifies disappearance; a still-present request blocks replacements this pass.
    private func cancelAndVerify(_ identifiers: Set<String>) async -> Bool {
        guard !identifiers.isEmpty else { return true }
        let sorted = identifiers.sorted()
        await center.removePending(identifiers: sorted)
        await center.removeDelivered(identifiers: sorted)
        do {
            let pending = try await center.pendingRequests()
            let delivered = try await center.deliveredRequests()
            let remaining = Set((pending + delivered).map(\.identifier)).intersection(identifiers)
            if remaining.isEmpty { return true }
            status.unconfirmedCancellationCount += remaining.count
        } catch {
            status.issues.insert(.observationsUnavailable)
            status.unconfirmedCancellationCount += identifiers.count
        }
        status.issues.insert(.cancellationUnconfirmed)
        return false
    }
}
