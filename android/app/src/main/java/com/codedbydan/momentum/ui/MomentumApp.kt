// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.ui

import com.codedbydan.momentum.R

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.adaptive.navigationsuite.NavigationSuiteScaffold
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLocale
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.codedbydan.momentum.*
import com.codedbydan.momentum.core.*
import com.codedbydan.momentum.data.AppSettings
import com.codedbydan.momentum.platform.PrivateClipboard
import com.codedbydan.momentum.platform.ClipboardUriUnsupportedException
import kotlinx.coroutines.launch
import kotlinx.coroutines.CancellationException
import androidx.compose.ui.input.key.*
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import java.time.LocalDate
import java.time.format.DateTimeFormatter

private data class OrganizationEdit(val project: Boolean, val id: String?, val title: String, val color: String?)

@OptIn(ExperimentalMaterial3Api::class)
@Composable fun MomentumApp(vm: MomentumViewModel) {
    val state by vm.state.collectAsStateWithLifecycle()
    MomentumTheme(state.snapshot?.settings ?: AppSettings()) {
        val drawer = rememberDrawerState(DrawerValue.Closed)
        val scope = rememberCoroutineScope()
        val context = LocalContext.current
        val locale = LocalLocale.current.platformLocale
        val clipboardTitle = stringResource(R.string.title)
        val clipboardFailure = stringResource(R.string.clipboard_copy_failed)
        val clipboardUnsupported = stringResource(R.string.clipboard_large_unsupported)
        val undoLabel = stringResource(R.string.undo)
        val notice = state.notice
        val noticeText = notice?.message?.let { messageText(it) }
            ?: if(notice?.undo != null) stringResource(R.string.action_changed) else null
        val snackbar = remember { SnackbarHostState() }
        var options by remember { mutableStateOf(false) }
        var rowMenu by remember { mutableStateOf<TaskRow?>(null) }
        var move by remember { mutableStateOf<List<String>?>(null) }
        var tag by remember { mutableStateOf<List<String>?>(null) }
        var organization by remember { mutableStateOf<OrganizationEdit?>(null) }
        var deletion by remember { mutableStateOf<Pair<SidebarEntry, Boolean>?>(null) }
        var deletionCount by remember { mutableStateOf<UInt?>(null) }
        var subtask by remember { mutableStateOf<String?>(null) }
        var repeat by remember { mutableStateOf<String?>(null) }
        var keyboardHelp by remember { mutableStateOf(false) }
        var searchFocusRevision by remember { mutableIntStateOf(0) }
        val searchFocus = remember { FocusRequester() }
        val keyboard = rememberAppKeyboardCommands(vm,
            modal = options || rowMenu != null || move != null || tag != null || organization != null ||
                deletion != null || subtask != null || repeat != null || drawer.isOpen || keyboardHelp,
            newProject = { organization = OrganizationEdit(true, null, "", null) },
            search = { vm.tab(Tab.Search); searchFocusRevision++ }, move = { move = it },
            repeat = { repeat = it }, help = { keyboardHelp = true })
        LaunchedEffect(state.tab, state.view, searchFocusRevision) { if(state.tab == Tab.Search) searchFocus.requestFocus() }
        LaunchedEffect(notice, noticeText, undoLabel) {
            val outcome = notice ?: return@LaunchedEffect
            val text = noticeText ?: run { vm.consumeNotice(outcome); return@LaunchedEffect }
            val result = snackbar.showSnackbar(text,
                actionLabel = if(outcome.undo != null) undoLabel else null,
                withDismissAction = true, duration = if(outcome.undo != null) SnackbarDuration.Long else SnackbarDuration.Short)
            if(result == SnackbarResult.ActionPerformed) outcome.undo?.let { id -> vm.action { undoBatch(id) } }
            vm.consumeNotice(outcome)
        }
        BackHandler(state.selecting) { vm.cancelSelection() }
        BackHandler(drawer.isOpen) { scope.launch { drawer.close() } }
        ModalNavigationDrawer(drawerState = drawer, drawerContent = {
            ModalDrawerSheet {
                LazyColumn(Modifier.fillMaxWidth()) {
                    item { Text(stringResource(R.string.lists), Modifier.padding(24.dp).semantics { heading() }, style = MaterialTheme.typography.headlineSmall) }
                    state.snapshot?.sidebar?.let { sidebar ->
                        (sidebar.fixed + sidebar.projects + sidebar.tags).forEach { entry ->
                            item("context-${entry.view}") {
                                val isProject = entry.view is View.Project
                                val isTag = entry.view is View.Tag
                                val selected = state.view == entry.view && state.tab != Tab.Settings
                                NavigationDrawerItem(selected = selected,
                                    onClick = { vm.navigate(entry.view); scope.launch { drawer.close() } },
                                    label = { Text(contextText(entry), maxLines = 2, overflow = TextOverflow.Ellipsis) },
                                    icon = { if(isProject || isTag) Icon(painterResource(if(isProject) R.drawable.ic_project else R.drawable.ic_tag), null,
                                        tint = nativeLabelInk(entry.color, selected)) },
                                    badge = {
                                        Row {
                                            if(entry.taskCount > 0u) Text(entry.taskCount.toString(), Modifier.padding(12.dp))
                                            if(isProject || isTag) {
                                                var expanded by remember { mutableStateOf(false) }
                                                IconButton({ expanded = true }) { Icon(Icons.Default.MoreVert, stringResource(R.string.list_actions, entry.title)) }
                                                DropdownMenu(expanded, { expanded = false }) {
                                                    DropdownMenuItem(text = { Text(stringResource(R.string.edit_task)) }, onClick = {
                                                        organization = OrganizationEdit(isProject, if(isProject) (entry.view as View.Project).id else (entry.view as View.Tag).id, entry.title, entry.color); expanded = false
                                                    })
                                                    DropdownMenuItem(text = { Text(stringResource(R.string.delete)) }, enabled = !(entry.view is View.Project && entry.view.id == "INBOX_PROJECT"), onClick = {
                                                        deletionCount = null; deletion = entry to isProject; expanded = false
                                                        val requested = entry to isProject
                                                        scope.launch { try {
                                                            val count = if(isProject) vm.repository.read { projectTaskCount((entry.view as View.Project).id) } else 0u
                                                            if(deletion == requested) deletionCount = count
                                                        } catch(e: Exception) { vm.reportError(e); if(deletion == requested) deletion = null } }
                                                    })
                                                }
                                            }
                                        }
                                    }, modifier = Modifier.padding(horizontal = 12.dp))
                            }
                        }
                    }
                    item { TextButton({ organization = OrganizationEdit(true, null, "", null) }, Modifier.padding(12.dp)) { Text(stringResource(R.string.new_project)) } }
                    item { TextButton({ organization = OrganizationEdit(false, null, "", null) }, Modifier.padding(12.dp)) { Text(stringResource(R.string.new_tag)) } }
                }
            }
        }) {
            NavigationSuiteScaffold(modifier = Modifier.appKeyboardShortcuts(keyboard), navigationSuiteItems = {
                Tab.entries.forEach { tab ->
                    val label = when(tab) { Tab.Today -> R.string.today; Tab.Upcoming -> R.string.upcoming; Tab.Search -> R.string.search; Tab.Settings -> R.string.settings }
                    item(selected = state.tab == tab, onClick = { vm.tab(tab) }, label = { AdaptiveNavigationLabel(stringResource(label)) },
                        icon = { Icon(when(tab) { Tab.Today -> Icons.Default.CheckCircle; Tab.Upcoming -> Icons.Default.DateRange; Tab.Search -> Icons.Default.Search; Tab.Settings -> Icons.Default.Settings }, null) })
                }
            }) {
                Scaffold(topBar = {
                    TopAppBar(expandedHeight = adaptiveTopAppBarHeight(), title = { AdaptiveTopAppBarTitle(when {
                        state.selecting -> pluralStringResource(R.plurals.task_selection_count, state.selected.size, state.selected.size)
                        state.tab == Tab.Settings -> stringResource(R.string.settings)
                        state.snapshot != null -> titleText(state.snapshot!!.listing.title)
                        else -> stringResource(R.string.app_name)
                    }) }, navigationIcon = {
                        if(state.tab != Tab.Settings) IconButton({ if(state.selecting) vm.cancelSelection() else scope.launch { drawer.open() } }, Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp)) {
                            Icon(if(state.selecting) Icons.Default.Close else Icons.Default.Menu, stringResource(if(state.selecting) R.string.finish_selection else R.string.lists))
                        }
                    }, actions = {
                        if(state.tab != Tab.Settings) {
                            IconButton(vm::refresh, Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp), enabled = !state.loading) { Icon(Icons.Default.Refresh, stringResource(R.string.refresh)) }
                            IconButton({ options = true }, Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp)) { Icon(Icons.Default.MoreVert, stringResource(R.string.options)) }
                        }
                    })
                }, snackbarHost = { SnackbarHost(snackbar) }, floatingActionButton = {
                    if(state.tab != Tab.Settings && state.view != View.Archive && !state.selecting) FloatingActionButton(onClick = { vm.newTask() }) {
                        Icon(Icons.Default.Add, stringResource(R.string.add_task))
                    }
                }, bottomBar = {
                    if(state.selecting) SelectionBar(state, vm, { move = state.selected.toList() }, { tag = state.selected.toList() })
                }) { padding ->
                    Column(Modifier.padding(padding)) {
                        if(state.editor == null) { CaptureStatus(state); DraftRecoveryStatus(state, vm) }
                        state.error?.let {
                            var details by remember(it) { mutableStateOf(false) }
                            val captureError = it == stringResource(R.string.capture_queue_full) ||
                                it == stringResource(R.string.capture_unlock_required)
                            Surface(color = MaterialTheme.colorScheme.errorContainer) {
                                Column(Modifier.fillMaxWidth().padding(16.dp).semantics { liveRegion = LiveRegionMode.Assertive }) {
                                    Text(stringResource(R.string.error_title), style = MaterialTheme.typography.titleMedium)
                                    if(details || captureError) Text(it)
                                    Row {
                                        if(!captureError) {
                                            TextButton({ if(state.editor?.isTextImport == true) vm.importEditorText() else if(state.editor != null) vm.saveEditor() else vm.refresh() }) { Text(stringResource(R.string.retry)) }
                                            TextButton({ details = !details }) { Text(stringResource(R.string.details)) }
                                        }
                                        TextButton(vm::clearError) { Text(stringResource(R.string.close)) }
                                    }
                                }
                            }
                        }
                        if(state.tab == Tab.Settings) SettingsScreen(state, vm)
                        else {
                            if(state.view == View.Search) OutlinedTextField(state.query, vm::search, label = { Text(stringResource(R.string.search)) },
                                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).focusRequester(searchFocus), singleLine = true,
                                trailingIcon = { if(state.query.isNotEmpty()) IconButton({ vm.search("") }) { Icon(Icons.Default.Close, stringResource(R.string.close)) } })
                            PullToRefreshBox(isRefreshing = state.loading && state.snapshot != null,
                                onRefresh = { if(state.snapshot?.settings?.provider?.let { it != com.codedbydan.momentum.data.SyncProvider.Off } == true) vm.platform.syncNow(); vm.refresh() }, modifier = Modifier.weight(1f)) {
                                TaskList(state, vm) { rowMenu = it }
                            }
                            SyncFooter(state, vm)
                        }
                    }
                }
            }
        }
        if(keyboardHelp) KeyboardHelp(state.snapshot?.settings?.keyboardModifier?.name ?: "Ctrl") { keyboardHelp = false }
        if(options) ViewOptions(state, vm, vm::openTextImport) { options = false }
        if(state.editor != null) TaskEditor(state, vm)
        rowMenu?.let { row ->
            ModalBottomSheet(onDismissRequest = { rowMenu = null }) {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                Text(row.title, Modifier.padding(20.dp), style = MaterialTheme.typography.titleLarge,
                    maxLines = 3, overflow = TextOverflow.Ellipsis)
                val ids = listOf(row.id)
                fun invoke(block: Engine.() -> Outcome) { vm.action(block = block); rowMenu = null }
                ActionRow(R.string.edit_task) { vm.edit(row.id); rowMenu = null }
                ActionRow(if(row.isDone) R.string.reopen else R.string.complete) { invoke { toggleDone(row.id) } }
                ActionRow(R.string.today) { invoke { moveToDay(ids, today()) } }
                ActionRow(R.string.morning) { invoke { toggleSlot(ids, Slot.MORNING) } }
                ActionRow(R.string.evening) { invoke { toggleSlot(ids, Slot.TONIGHT) } }
                ActionRow(R.string.tomorrow) { invoke { moveToTomorrow(ids) } }
                listOf(2L, 3L).forEach { offset ->
                    val day = LocalDate.now().plusDays(offset)
                    val name = day.format(DateTimeFormatter.ofPattern("EEEE", locale))
                    ActionRow(name) { invoke { moveToDay(ids, day.toString()) } }
                }
                ActionRow(R.string.next_week) { invoke { moveToNextWeek(ids) } }
                ActionRow(R.string.move_to) { move = ids; rowMenu = null }
                ActionRow(R.string.add_tag) { tag = ids; rowMenu = null }
                ActionRow(R.string.move_up) { invoke { nudge(row.id, -1, state.view) } }
                ActionRow(R.string.move_down) { invoke { nudge(row.id, 1, state.view) } }
                if(!row.isSubtask) {
                    ActionRow(R.string.subtask) { subtask = row.id; rowMenu = null }
                    ActionRow(R.string.repeat) { repeat = row.id; rowMenu = null }
                }
                ActionRow(R.string.duplicate) { invoke { duplicateTask(row.id) } }
                ActionRow(R.string.copy_title) {
                    scope.launch {
                        try { PrivateClipboard.copy(context, clipboardTitle, row.title) }
                        catch(e: CancellationException) { throw e }
                        catch(_: ClipboardUriUnsupportedException) { vm.reportError(IllegalStateException(clipboardUnsupported)) }
                        catch(_: Exception) { vm.reportError(IllegalStateException(clipboardFailure)) }
                    }
                    rowMenu = null
                }
                ActionRow(R.string.delete) { invoke { deleteTask(row.id) } }
                Spacer(Modifier.height(24.dp))
                }
            }
        }
        move?.let { ids -> AlertDialog(onDismissRequest = { move = null }, title = { Text(stringResource(R.string.move_to)) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    state.snapshot?.projects.orEmpty().forEach { project ->
                        TextButton({ vm.action { moveToProject(ids, project.id) }; move = null }) {
                            Text(project.title, maxLines = 2, overflow = TextOverflow.Ellipsis)
                        }
                    }
                }
            },
            confirmButton = { TextButton({ move = null }) { Text(stringResource(R.string.cancel)) } }) }
        tag?.let { ids -> TextEntryDialog(R.string.add_tag, onDismiss = { tag = null }) { name -> vm.action { addTagByName(ids, name) }; tag = null } }
        subtask?.let { id -> TextEntryDialog(R.string.subtask, onDismiss = { subtask = null }) { title -> vm.action { addSubtask(id, title) }; subtask = null } }
        repeat?.let { id -> RepeatEditor(id, vm) { repeat = null } }
        organization?.let { edit ->
            var title by remember(edit) { mutableStateOf(edit.title) }
            var color by remember(edit) { mutableStateOf(edit.color.orEmpty()) }
            var entity by remember(edit) { mutableStateOf(edit.id) }
            AlertDialog(onDismissRequest = { organization = null }, title = { Text(stringResource(if(edit.project) R.string.project else R.string.tags)) },
                text = { Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    OutlinedTextField(title, { title = it }, label = { Text(stringResource(R.string.name)) }, singleLine = true)
                    OutlinedTextField(color, { color = it }, label = { Text(stringResource(R.string.color)) }, placeholder = { Text("#RRGGBB") }, singleLine = true)
                    if(state.error != null) Text(stringResource(R.string.organization_failed), color = MaterialTheme.colorScheme.error)
                } }, confirmButton = { TextButton({ vm.organization(edit.project, entity, title.trim(), color.takeIf { it.isNotBlank() }, onEntity = { entity = it }, onSuccess = { organization = null }) },
                    enabled = title.isNotBlank() && (color.isBlank() || color.matches(Regex("#[0-9a-fA-F]{6}")))) { Text(stringResource(R.string.save)) } },
                dismissButton = { TextButton({ organization = null }) { Text(stringResource(R.string.cancel)) } })
        }
        deletion?.let { (entry, project) -> AlertDialog(onDismissRequest = { deletion = null }, title = {
            Text(stringResource(R.string.delete_organization, entry.title), maxLines = 2, overflow = TextOverflow.Ellipsis)
        }, text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                if(deletionCount == null) CircularProgressIndicator() else Text(if(project)
                    pluralStringResource(R.plurals.delete_project_scope, deletionCount!!.toInt(), deletionCount!!.toInt())
                    else stringResource(R.string.delete_tag_scope))
            }
        },
            confirmButton = { TextButton({ vm.action { if(project) deleteProject((entry.view as View.Project).id) else deleteTag((entry.view as View.Tag).id) }; deletion = null }, enabled = deletionCount != null) { Text(stringResource(R.string.delete)) } },
            dismissButton = { TextButton({ deletion = null }) { Text(stringResource(R.string.cancel)) } }) }
    }
}

@Composable private fun contextText(entry: SidebarEntry): String = when(entry.view) {
    View.Today -> stringResource(R.string.today); View.Morning -> stringResource(R.string.morning); View.Tonight -> stringResource(R.string.evening)
    View.Upcoming -> stringResource(R.string.upcoming); View.Archive -> stringResource(R.string.archive); View.Search -> stringResource(R.string.search)
    is View.Project, is View.Tag -> entry.title
}
@Composable fun ActionRow(label: Int, click: () -> Unit) {
    ActionRow(stringResource(label), click)
}
@Composable fun ActionRow(label: String, click: () -> Unit) {
    ListItem(headlineContent = { Text(label) }, modifier = Modifier.fillMaxWidth().clickable(onClick = click))
}
@OptIn(ExperimentalLayoutApi::class)
@Composable private fun SelectionBar(state: UiState, vm: MomentumViewModel, move: () -> Unit, tag: () -> Unit) {
    val ids = state.selected.toList()
    val locale = LocalLocale.current.platformLocale
    var more by remember { mutableStateOf(false) }
    Surface(tonalElevation = 3.dp) {
        FlowRow(Modifier.fillMaxWidth().padding(8.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            TextButton(vm::selectAll) { Text(stringResource(R.string.select_all)) }
            TextButton({ vm.action { bulkDone(ids) } }, enabled = ids.isNotEmpty()) { Text(stringResource(R.string.complete)) }
            TextButton(move, enabled = ids.isNotEmpty()) { Text(stringResource(R.string.move_to)) }
            Box {
                IconButton({ more = true }, enabled = ids.isNotEmpty()) { Icon(Icons.Default.MoreVert, stringResource(R.string.selection_actions)) }
                DropdownMenu(more, { more = false }) {
                    fun invoke(block: Engine.() -> Outcome) { more = false; vm.action(block = block) }
                    DropdownMenuItem({ Text(stringResource(R.string.today)) }, { invoke { planForToday(ids) } })
                    DropdownMenuItem({ Text(stringResource(R.string.morning)) }, { invoke { toggleSlot(ids, Slot.MORNING) } })
                    DropdownMenuItem({ Text(stringResource(R.string.evening)) }, { invoke { toggleSlot(ids, Slot.TONIGHT) } })
                    DropdownMenuItem({ Text(stringResource(R.string.tomorrow)) }, { invoke { moveToTomorrow(ids) } })
                    listOf(2L, 3L).forEach { offset ->
                        val day = LocalDate.now().plusDays(offset)
                        val name = day.format(DateTimeFormatter.ofPattern("EEEE", locale))
                        DropdownMenuItem({ Text(name) }, { invoke { moveToDay(ids, day.toString()) } })
                    }
                    DropdownMenuItem({ Text(stringResource(R.string.next_week)) }, { invoke { moveToNextWeek(ids) } })
                    DropdownMenuItem({ Text(stringResource(R.string.add_tag)) }, { more = false; tag() })
                    DropdownMenuItem({ Text(stringResource(R.string.delete)) }, { invoke { bulkDelete(ids) } })
                }
            }
        }
    }
}
