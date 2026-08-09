<script lang="ts">
	import { onMount } from 'svelte';
	import Editor from './lib/Editor.svelte';
	import ContextMenu from './lib/ContextMenu.svelte';
	import SettingsModal from './lib/SettingsModal.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open, save as showSaveDialog, confirm as showConfirm } from '@tauri-apps/plugin-dialog';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	type Theme = 'dark' | 'light' | 'system';
	interface EditorExposed {
		hasSelection: () => boolean;
		handleCut: () => Promise<void>;
		handleCopy: () => Promise<void>;
		handlePaste: () => Promise<void>;
		undo: () => void;
		redo: () => void;
		handleSelectAll: () => void;
		transformSelection: (type: 'lowercase' | 'uppercase' | 'propercase') => void;
		markSaved: () => void;
		isDirty: () => boolean;
		getContent: () => string;
		recheckSpelling: () => void;
		loadContent: (content: string) => void;
	}

	const FILTERS = [{ name: 'Markdown', extensions: ['md', 'markdown', 'txt'] }];

	let editorRef = $state<EditorExposed | null>(null);
	let currentPath = $state<string | null>(null);
	let showSettings = $state(false);
	let theme = $state<Theme>('dark');
	let fontSize = $state(18);
	let fontFamily = $state("'SF Mono', 'Fira Code', 'Cascadia Code', monospace");
	let wordWrap = $state(true);
	let spellcheck = $state(true);
	let resolvedTheme = $state<'dark' | 'light'>('dark');

	let ctxMenu = $state<{ show: boolean; x: number; y: number; items: any[] }>({
		show: false,
		x: 0,
		y: 0,
		items: [],
	});

	// ─── menu bar (in-window HTML — native OS menus can't be resized) ───

	let openMenu = $state<number | null>(null);

	const menus: {
		label: string;
		items: {
			label?: string;
			separator?: boolean;
			action?: () => void;
			disabled?: boolean | (() => boolean);
		}[];
	}[] = [
		{
			label: 'File',
			items: [
				{ label: 'New', action: () => newFile() },
				{ label: 'Open...', action: () => openFile() },
				{ label: 'Settings...', action: () => openSettings() },
				{ separator: true },
				{ label: 'Save', action: () => saveFile() },
				{ label: 'Save As...', action: () => saveFileAs() },
				{ separator: true },
				{ label: 'Close Window', action: () => getCurrentWindow().close() },
				{ label: 'Exit', action: () => getCurrentWindow().close() },
			],
		},
		{
			label: 'Edit',
			items: [
				{ label: 'Undo', action: () => editorRef?.undo() },
				{ label: 'Redo', action: () => editorRef?.redo() },
				{ separator: true },
				{ label: 'Cut', action: () => editorRef?.handleCut() },
				{ label: 'Copy', action: () => editorRef?.handleCopy() },
				{ label: 'Paste', action: () => editorRef?.handlePaste() },
				{ separator: true },
				{ label: 'Select All', action: () => editorRef?.handleSelectAll() },
				{ separator: true },
				{
					label: 'lowercase',
					disabled: () => !(editorRef?.hasSelection() ?? false),
					action: () => editorRef?.transformSelection('lowercase'),
				},
				{
					label: 'UPPERCASE',
					disabled: () => !(editorRef?.hasSelection() ?? false),
					action: () => editorRef?.transformSelection('uppercase'),
				},
				{
					label: 'Title Case',
					disabled: () => !(editorRef?.hasSelection() ?? false),
					action: () => editorRef?.transformSelection('propercase'),
				},
			],
		},
		{
			label: 'View',
			items: [
				{
					label: 'Zoom In',
					action: () => handleSettingsChange({ fontSize: Math.min(32, fontSize + 1) }),
				},
				{
					label: 'Zoom Out',
					action: () => handleSettingsChange({ fontSize: Math.max(10, fontSize - 1) }),
				},
				{ label: 'Reset Zoom', action: () => handleSettingsChange({ fontSize: 18 }) },
				{ separator: true },
				{
					label: 'Toggle Theme',
					action: () =>
						handleSettingsChange({
							theme: resolvedTheme === 'dark' ? 'light' : 'dark',
						}),
				},
				{
					label: 'Toggle Word Wrap',
					action: () => handleSettingsChange({ wordWrap: !wordWrap }),
				},
				{
					label: 'Toggle Spell Check',
					action: () => handleSettingsChange({ spellcheck: !spellcheck }),
				},
			],
		},
	];

	// ─── helpers ────────────────────────────────────────

	function isDirty() {
		return editorRef?.isDirty() ?? false;
	}

	function getContent() {
		return editorRef?.getContent() ?? '';
	}

	function updateTitle() {
		try {
			const name = currentPath
				? currentPath.split('/').pop() || currentPath.split('\\').pop()
				: 'Untitled';
			const win = getCurrentWindow();
			win.setTitle(`Bearpad 2 — ${name}${isDirty() ? ' ●' : ''}`);
		} catch {
			/* title is cosmetic; never let it break the mount chain */
		}
	}

	// ─── file operations ─────────────────────────────────

	async function newFile() {
		if (isDirty()) {
			const ok = await showConfirm('Discard unsaved changes?', {
				title: 'Bearpad 2',
				kind: 'warning',
			});
			if (!ok) return;
		}
		editorRef?.loadContent('');
		currentPath = null;
		updateTitle();
	}

	async function openFile() {
		if (isDirty()) {
			const ok = await showConfirm('Discard unsaved changes?', {
				title: 'Bearpad 2',
				kind: 'warning',
			});
			if (!ok) return;
		}
		const selected = await open({ filters: FILTERS, multiple: false });
		if (!selected) return;
		const path = selected as string;
		try {
			const content = await invoke<string>('read_file', { path });
			editorRef?.loadContent(content);
			currentPath = path;
			updateTitle();
		} catch (e) {
			console.error('Failed to open file:', e);
		}
	}

	async function saveFile() {
		if (currentPath) {
			const content = getContent();
			try {
				await invoke('write_file', { path: currentPath, content });
				editorRef?.markSaved();
				updateTitle();
			} catch (e) {
				console.error('Failed to save file:', e);
			}
		} else {
			await saveFileAs();
		}
	}

	async function saveFileAs() {
		const selected = await showSaveDialog({ filters: FILTERS, defaultPath: 'untitled.md' });
		if (!selected) return;
		const path = selected as string;
		const content = getContent();
		try {
			await invoke('write_file', { path, content });
			currentPath = path;
			editorRef?.markSaved();
			updateTitle();
		} catch (e) {
			console.error('Failed to save file:', e);
		}
	}

	// ─── settings ────────────────────────────────────────

	async function loadSettings() {
		try {
			const data = await invoke<string>('read_settings');
			const s = JSON.parse(data);
			if (s.theme) theme = s.theme;
			if (s.fontSize != null) fontSize = s.fontSize;
			if (s.fontFamily) fontFamily = s.fontFamily;
			if (s.wordWrap != null) wordWrap = s.wordWrap;
			if (s.spellcheck != null) spellcheck = s.spellcheck;
		} catch { /* defaults */ }
		resolveTheme();
	}

	async function saveSettings() {
		try {
			await invoke('write_settings', {
				json: JSON.stringify({ theme, fontSize, fontFamily, wordWrap, spellcheck }),
			});
		} catch (e) {
			console.error('Failed to save settings:', e);
		}
	}

	function resolveTheme() {
		if (theme === 'light') resolvedTheme = 'light';
		else if (theme === 'system') {
			resolvedTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
				? 'dark'
				: 'light';
		} else {
			resolvedTheme = 'dark';
		}
		// body needs the attr too — theme CSS vars are keyed on body[data-theme]
		document.body.dataset.theme = resolvedTheme;
	}

	function handleSettingsChange(
		patch: Partial<{ theme: Theme; fontSize: number; fontFamily: string; wordWrap: boolean; spellcheck: boolean }>,
	) {
		if (patch.theme !== undefined) theme = patch.theme;
		if (patch.fontSize !== undefined) fontSize = patch.fontSize;
		if (patch.fontFamily !== undefined) fontFamily = patch.fontFamily;
		if (patch.wordWrap !== undefined) wordWrap = patch.wordWrap;
		if (patch.spellcheck !== undefined) spellcheck = patch.spellcheck;
		resolveTheme();
		saveSettings();
	}

	function openSettings() {
		showSettings = true;
	}

	function closeSettings() {
		showSettings = false;
	}

	// ─── menu bar ────────────────────────────────────────

	onMount(async () => {
		await loadSettings();


		// Close confirmation lives in Rust (on_window_event + native dialog +
		// destroy) — the JS dialog path hangs on Windows in every variant.

		// System theme listener
		window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
			if (theme === 'system') resolveTheme();
		});

		updateTitle();
	});

	// ─── context menu handler ───────────────────────────

	function onContextMenu(e: MouseEvent) {
		e.preventDefault();
		const isMac = /mac/i.test(navigator.platform);
		const mod = isMac ? '⌘' : 'Ctrl+';
		const isInsideEditor = !!(e.target as HTMLElement).closest('.cm-editor');
		const hasSelection = editorRef?.hasSelection() || false;

		const items: any[] = [];

		if (isInsideEditor) {
			if (hasSelection) {
				items.push(
					{ label: 'Cut', shortcut: `${mod}X`, onClick: () => editorRef?.handleCut() },
					{ label: 'Copy', shortcut: `${mod}C`, onClick: () => editorRef?.handleCopy() },
				);
			}
			items.push(
				{ label: 'Paste', shortcut: `${mod}V`, onClick: () => editorRef?.handlePaste() },
				{ separator: true },
				{ label: 'Undo', shortcut: `${mod}Z`, onClick: () => editorRef?.undo() },
				{ label: 'Redo', shortcut: `${isMac ? '⇧⌘' : 'Ctrl+Shift+'}Z`, onClick: () => editorRef?.redo() },
				{ separator: true },
				{
					label: 'lowercase',
					disabled: !hasSelection,
					onClick: () => {
						editorRef?.transformSelection('lowercase');
						hideMenu();
					},
				},
				{
					label: 'UPPERCASE',
					disabled: !hasSelection,
					onClick: () => {
						editorRef?.transformSelection('uppercase');
						hideMenu();
					},
				},
				{
					label: 'Title Case',
					disabled: !hasSelection,
					onClick: () => {
						editorRef?.transformSelection('propercase');
						hideMenu();
					},
				},
				{ separator: true },
			);

			// right-clicking a misspelled word offers to learn it
			const errWord = (e.target as HTMLElement)
				.closest('.cm-spell-error')
				?.textContent?.trim();
			if (errWord) {
				items.push(
					{ separator: true },
					{
						label: `Add "${errWord}" to dictionary`,
						onClick: async () => {
							try {
								await invoke('add_to_dictionary', { word: errWord });
								editorRef?.recheckSpelling();
							} catch {
								/* ignore */
							}
							hideMenu();
						},
					},
				);
			}
		}

		items.push({
			label: 'Select All',
			shortcut: `${mod}A`,
			onClick: () => editorRef?.handleSelectAll(),
		});

		ctxMenu = { show: true, x: e.clientX, y: e.clientY, items };
	}

	function hideMenu() {
		ctxMenu.show = false;
	}

	// ─── editor ready ───────────────────────────────────

	function onEditorReady(ref: EditorExposed) {
		editorRef = ref;
		updateTitle();
	}

	// Update title periodically for dirty indicator
	let titleTimer: ReturnType<typeof setInterval>;
	onMount(() => {
		titleTimer = setInterval(() => updateTitle(), 500);

		// Context menu listener
		document.addEventListener('contextmenu', onContextMenu as EventListener);

		// View shortcuts handled in-page: Windows WebView2 swallows OS menu
		// accelerators when the webview has focus (and hijacks Ctrl+= as browser
		// zoom), so keydown is the one path that behaves identically everywhere.
		window.addEventListener('keydown', (e) => {
			const k = e.key;
			const mod = e.metaKey || e.ctrlKey;
			if (mod && !e.altKey) {
				if (k === '=' || k === '+') {
					e.preventDefault();
					handleSettingsChange({ fontSize: Math.min(32, fontSize + 1) });
				} else if (k === '-' || k === '_') {
					e.preventDefault();
					handleSettingsChange({ fontSize: Math.max(10, fontSize - 1) });
				} else if (k === '0') {
					e.preventDefault();
					handleSettingsChange({ fontSize: 18 });
				} else if (k === '\\') {
					e.preventDefault();
					handleSettingsChange({
						theme: resolvedTheme === 'dark' ? 'light' : 'dark',
					});
				} else if (k === 'n' || k === 'N') {
					e.preventDefault();
					newFile();
				} else if (k === 'o' || k === 'O') {
					e.preventDefault();
					openFile();
				} else if (k === 's' || k === 'S') {
					e.preventDefault();
					if (e.shiftKey) saveFileAs();
					else saveFile();
				} else if (k === 'w' || k === 'W' || k === 'q' || k === 'Q') {
					e.preventDefault();
					getCurrentWindow().close();
				} else if (k === ',') {
					e.preventDefault();
					openSettings();
				}
			} else if (e.altKey && !mod && (k === 'z' || k === 'Z')) {
				e.preventDefault();
				handleSettingsChange({ wordWrap: !wordWrap });
			}
		});
		document.addEventListener('click', (e) => {
			if (
				ctxMenu.show &&
				!(e.target as HTMLElement).closest('.custom-context-menu')
			) {
				hideMenu();
			}
			if (openMenu !== null && !(e.target as HTMLElement).closest('.menu-bar')) {
				openMenu = null;
			}
		});
	});
</script>

<div class="app-root" data-theme={resolvedTheme}>
	<div class="menu-bar" role="menubar">
		{#each menus as menu, i (menu.label)}
			<div
				class="menu-item"
				class:open={openMenu === i}
				role="menuitem"
				tabindex="-1"
				onmouseenter={() => (openMenu !== null ? (openMenu = i) : null)}
				onclick={(e) => {
					e.stopPropagation();
					openMenu = openMenu === i ? null : i;
				}}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') {
						e.preventDefault();
						e.stopPropagation();
						openMenu = openMenu === i ? null : i;
					}
				}}
			>
				<span class="menu-label">{menu.label}</span>
				{#if openMenu === i}
					<div class="menu-dropdown" role="menu">
						{#each menu.items as item}
							{@const disabled = typeof item.disabled === 'function' ? item.disabled() : item.disabled}
							{#if item.separator}
								<div class="menu-sep"></div>
							{:else}
								<button
									class="menu-action"
									class:menu-disabled={disabled}
									role="menuitem"
									disabled={disabled}
									onclick={(e) => {
										e.stopPropagation();
										if (disabled) return;
										item.action?.();
										openMenu = null;
									}}
								>
									{item.label}
								</button>
							{/if}
						{/each}
					</div>
				{/if}
			</div>
		{/each}
	</div>

	<div class="editor-wrap">
		<Editor
			onReady={onEditorReady}
			theme={resolvedTheme}
			{fontSize}
			{fontFamily}
			{wordWrap}
			{spellcheck}
		/>
	</div>

	{#if ctxMenu.show}
		<ContextMenu contextMenu={ctxMenu} onhide={hideMenu} />
	{/if}

	{#if showSettings}
		<SettingsModal
			settings={{ theme, fontSize, fontFamily, wordWrap }}
			onChange={handleSettingsChange}
			onClose={closeSettings}
		/>
	{/if}
</div>

<style>
	:global(body) {
		margin: 0;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
		background: #1e1e1e;
		color: #d4d4d4;
		--menu-bg: #252526;
		--menu-border: #3c3c3c;
		--menu-hover: #37373d;
		--menu-sep: #3c3c3c;
		--menu-text: #d4d4d4;
	}
	:global(body[data-theme="light"]) {
		background: #ffffff;
		color: #333333;
		--menu-bg: #f3f3f3;
		--menu-border: #d4d4d4;
		--menu-hover: #e2e2e2;
		--menu-sep: #d4d4d4;
		--menu-text: #1a1a1a;
	}
	.app-root {
		display: flex;
		flex-direction: column;
		height: 100vh;
	}
	.menu-bar {
		display: flex;
		align-items: stretch;
		background: var(--menu-bg);
		color: var(--menu-text);
		border-bottom: 1px solid var(--menu-border);
		user-select: none;
		position: relative;
		z-index: 1000;
		flex-shrink: 0;
	}
	.menu-item {
		position: relative;
	}
	.menu-label {
		display: block;
		padding: 8px 14px;
		font-size: 16px;
		cursor: default;
	}
	.menu-item.open .menu-label,
	.menu-item:hover .menu-label {
		background: var(--menu-hover);
	}
	.menu-dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		min-width: 230px;
		background: var(--menu-bg);
		border: 1px solid var(--menu-border);
		box-shadow: 0 6px 16px rgba(0, 0, 0, 0.4);
		padding: 4px 0;
	}
	.menu-action {
		display: block;
		width: 100%;
		text-align: left;
		padding: 5px 16px;
		font-size: 16px;
		background: none;
		border: none;
		color: var(--menu-text);
		cursor: default;
	}
	.menu-action:hover {
		background: #094771;
		color: #ffffff;
	}
	.menu-action.menu-disabled,
	.menu-action:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.menu-action:disabled:hover {
		background: none;
		color: var(--menu-text);
	}
	.menu-sep {
		height: 1px;
		background: var(--menu-sep);
		margin: 4px 8px;
	}
	.editor-wrap {
		flex: 1;
		min-height: 0;
	}
	:global(.editor-host) {
		height: 100%;
	}
	:global(.cm-editor) {
		height: 100%;
	}
	.app-root[data-theme="light"] .menu-bar {
		background: #f3f3f3;
		border-bottom-color: #d8d8d8;
	}
	.app-root[data-theme="light"] .menu-item.open .menu-label,
	.app-root[data-theme="light"] .menu-item:hover .menu-label {
		background: #e6e6e6;
	}
	.app-root[data-theme="light"] .menu-dropdown {
		background: #f3f3f3;
		border-color: #d0d0d0;
	}
	.app-root[data-theme="light"] .menu-action:hover {
		background: #0078d4;
		color: #ffffff;
	}
	.app-root[data-theme="light"] .menu-sep {
		background: #d8d8d8;
	}
</style>