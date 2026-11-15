# Clean Up Inbox

The built-in Inbox on Linux, Android, iOS and macOS offers a Clean Up Inbox button.
Review the proposed task titles, links and tags before applying selected changes.
Momentum applies the selection as one undoable edit.

The shared core cleans up whitespace, capitalization and repeated punctuation. It
preserves existing task notes and moves web links from titles into notes without
duplicates. For link-only tasks, it derives a title from the host and readable path.
It suggests existing matching tags and leaves scheduling tags alone. Completed
tasks and tasks in other projects are outside this action.

On supported iOS and macOS devices, Apple Intelligence can refine the title and
suggest existing tags before review. The model runs on the device and receives
the locally cleaned title, link domains and eligible tag names. Momentum
keeps the rule-based suggestions if the model is unavailable or cannot produce
a valid result. The action never fetches links or contacts an AI service.

If a selected task changes after you open the preview, Momentum asks you to review
a fresh preview before applying changes.
