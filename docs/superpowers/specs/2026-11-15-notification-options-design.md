# Notification options

F-007, F-032 and F-038 cover this change across iOS, macOS, Android and Linux.

- Add Overdue Only to the existing task count choices. Count unfinished top-level tasks using the same date rule as Overdue.
- Keep summary opt-in and local time. Add independently selectable weekdays and Today, Morning, Evening and Overdue sections. Existing installs default to every day and the existing three sections. A date outside the selection or a summary with no matching open tasks sends nothing.
- Show a preview based on local tasks. Store preferences locally. Preserve the existing once-per-day claims, scheduling horizon, permission handling and individual reminder actions.
- Offer independent notification badge options for task reminders and summaries where the OS supports it. iOS uses the selected count captured when the app plans the notification. macOS can refresh the live Dock count. Android uses notification channels and the launcher controls presentation. Linux uses its Background Apps status for the count.
- Verify shared rules, compatibility with old preferences and notification data, native settings and delivery. Record the checks run and any platform limitations.
