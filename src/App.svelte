<script lang="ts">
	import { onMount } from 'svelte';
	import Editor from './lib/Editor.svelte';
	import ContextMenu from './lib/ContextMenu.svelte';
	import SettingsModal from './lib/SettingsModal.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open, save as showSaveDialog, confirm as showConfirm } from '@tauri-apps/plugin-dialog';
	import { Menu, Submenu, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu';
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
		loadContent: (content: string) => void;
	}

	const FILTERS = [{ name: 'Markdown', extensions: ['md', 'markdown', 'txt'] }];

	let editorRef = $state<EditorExposed | null>(null);
	let currentPath = $state<string | null>(null);
	let showSettings = $state(false);
	let theme = $state<Theme>('dark');
	let fontSize = $state(14);
	let fontFamily = $state("'SF Mono', 'Fira Code', 'Cascadia Code', monospace");
	let resolvedTheme = $state<'dark' | 'light'>('dark');

	let ctxMenu = $state<{ show: boolean; x: number; y: number; items: any[] }>({
		show: false,
		x: 0,
		y: 0,
		items: [],
	});

	// ─── helpers ────────────────────────────────────────

	function isDirty() {
		return editorRef?.isDirty() ?? false;
	}

	function getContent() {
		return editorRef?.getContent() ?? '';
	}

	function updateTitle() {
		const name = currentPath
			? currentPath.split('/').pop() || currentPath.split('\\').pop()
			: 'Untitled';
		const win = getCurrentWindow();
		win.setTitle(`Bearpad 2 — ${name}${isDirty() ? ' ●' : ''}`);
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
			await invoke('write_file', { path: currentPath, content });
			editorRef?.markSaved();
			updateTitle();
		} else {
			await saveFileAs();
		}
	}

	async function saveFileAs() {
		const selected = await showSaveDialog({ filters: FILTERS, defaultPath: 'untitled.md' });
		if (!selected) return;
		const path = selected as string;
		const content = getContent();
		await invoke('write_file', { path, content });
		currentPath = path;
		editorRef?.markSaved();
		updateTitle();
	}

	// ─── settings ────────────────────────────────────────

	async function loadSettings() {
		try {
			const data = await invoke<string>('read_settings');
			const s = JSON.parse(data);
			if (s.theme) theme = s.theme;
			if (s.fontSize != null) fontSize = s.fontSize;
			if (s.fontFamily) fontFamily = s.fontFamily;
		} catch { /* defaults */ }
		resolveTheme();
	}

	async function saveSettings() {
		try {
			await invoke('write_settings', {
				json: JSON.stringify({ theme, fontSize, fontFamily }),
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
	}

	function handleSettingsChange(
		patch: Partial<{ theme: Theme; fontSize: number; fontFamily: string }>,
	) {
		if (patch.theme !== undefined) theme = patch.theme;
		if (patch.fontSize !== undefined) fontSize = patch.fontSize;
		if (patch.fontFamily !== undefined) fontFamily = patch.fontFamily;
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

		// Build native menu items
		const settingsItem = await MenuItem.new({
			text: 'Settings...',
			accelerator: 'CmdOrCtrl+,',
			action: () => openSettings(),
		});

		const appSub = await Submenu.new({
			text: '',
			items: [
				await PredefinedMenuItem.new({ item: { About: null } }),
				await PredefinedMenuItem.new({ item: 'Separator' }),
				settingsItem,
				await PredefinedMenuItem.new({ item: 'Separator' }),
				await PredefinedMenuItem.new({ item: 'Services' }),
				await PredefinedMenuItem.new({ item: 'Separator' }),
				await PredefinedMenuItem.new({ item: 'Hide' }),
				await PredefinedMenuItem.new({ item: 'HideOthers' }),
				await PredefinedMenuItem.new({ item: 'ShowAll' }),
				await PredefinedMenuItem.new({ item: 'Separator' }),
				await PredefinedMenuItem.new({ item: 'Quit' }),
			],
		});

		const fileSub = await Submenu.new({
			text: 'File',
			items: [
				await MenuItem.new({
					text: 'New',
					accelerator: 'CmdOrCtrl+N',
					action: () => newFile(),
				}),
				await MenuItem.new({
					text: 'Open...',
					accelerator: 'CmdOrCtrl+O',
					action: () => openFile(),
				}),
				await PredefinedMenuItem.new({ item: 'Separator' }),
				await MenuItem.new({
					text: 'Save',
					accelerator: 'CmdOrCtrl+S',
					action: () => saveFile(),
				}),
				await MenuItem.new({
					text: 'Save As...',
					accelerator: 'CmdOrCtrl+Shift+S',
					action: () => saveFileAs(),
				}),
				await PredefinedMenuItem.new({ item: 'Separator' }),
				await MenuItem.new({
					text: 'Close Window',
					accelerator: 'CmdOrCtrl+W',
					action: () => getCurrentWindow().close(),
				}),
			],
		});

		const editSub = await Submenu.new({
			text: 'Edit',
			items: [
				await MenuItem.new({
					text: 'Undo',
					accelerator: 'CmdOrCtrl+Z',
					action: () => editorRef?.undo(),
				}),
				await MenuItem.new({
					text: 'Redo',
					accelerator: 'CmdOrCtrl+Shift+Z',
					action: () => editorRef?.redo(),
				}),
				await PredefinedMenuItem.new({ item: 'Separator' }),
				await MenuItem.new({
					text: 'Cut',
					accelerator: 'CmdOrCtrl+X',
					action: () => editorRef?.handleCut(),
				}),
				await MenuItem.new({
					text: 'Copy',
					accelerator: 'CmdOrCtrl+C',
					action: () => editorRef?.handleCopy(),
				}),
				await MenuItem.new({
					text: 'Paste',
					accelerator: 'CmdOrCtrl+V',
					action: () => editorRef?.handlePaste(),
				}),
				await PredefinedMenuItem.new({ item: 'Separator' }),
				await MenuItem.new({
					text: 'Select All',
					accelerator: 'CmdOrCtrl+A',
					action: () => editorRef?.handleSelectAll(),
				}),
			],
		});

		const viewSub = await Submenu.new({
			text: 'View',
			items: [
				await MenuItem.new({
					text: 'Zoom In',
					accelerator: 'CmdOrCtrl+=',
					action: () => handleSettingsChange({ fontSize: Math.min(32, fontSize + 1) }),
				}),
				await MenuItem.new({
					text: 'Zoom Out',
					accelerator: 'CmdOrCtrl+-',
					action: () => handleSettingsChange({ fontSize: Math.max(10, fontSize - 1) }),
				}),
				await MenuItem.new({
					text: 'Reset Zoom',
					accelerator: 'CmdOrCtrl+0',
					action: () => handleSettingsChange({ fontSize: 14 }),
				}),
				await PredefinedMenuItem.new({ item: 'Separator' }),
				await MenuItem.new({
					text: 'Toggle Theme',
					accelerator: 'CmdOrCtrl+\\',
					action: () => {
						handleSettingsChange({
							theme: resolvedTheme === 'dark' ? 'light' : 'dark',
						});
					},
				}),
			],
		});

		const menu = await Menu.new({ items: [appSub, fileSub, editSub, viewSub] });
		await menu.setAsAppMenu();

		// Close confirmation
		const win = getCurrentWindow();
		win.onCloseRequested(async (event) => {
			if (isDirty()) {
				event.preventDefault();
				const ok = await showConfirm('You have unsaved changes. Discard and close?', {
					title: 'Bearpad 2',
					kind: 'warning',
				});
				if (ok) win.close();
			}
		});

		// System theme listener
		window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
			if (theme === 'system') resolveTheme();
		});

		updateTitle();
	});

	// ─── context menu handler ───────────────────────────

	function onContextMenu(e: MouseEvent) {
		e.preventDefault();
		const isInsideEditor = !!(e.target as HTMLElement).closest('.cm-editor');
		const hasSelection = editorRef?.hasSelection() || false;

		const items: any[] = [];

		if (isInsideEditor) {
			if (hasSelection) {
				items.push(
					{ label: 'Cut', shortcut: '⌘X', onClick: () => editorRef?.handleCut() },
					{ label: 'Copy', shortcut: '⌘C', onClick: () => editorRef?.handleCopy() },
				);
			}
			items.push(
				{ label: 'Paste', shortcut: '⌘V', onClick: () => editorRef?.handlePaste() },
				{ separator: true },
				{ label: 'Undo', shortcut: '⌘Z', onClick: () => editorRef?.undo() },
				{ label: 'Redo', shortcut: '⇧⌘Z', onClick: () => editorRef?.redo() },
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
		}

		items.push({
			label: 'Select All',
			shortcut: '⌘A',
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
		document.addEventListener('click', (e) => {
			if (
				ctxMenu.show &&
				!(e.target as HTMLElement).closest('.custom-context-menu')
			) {
				hideMenu();
			}
		});
	});
</script>

<div class="app-root" data-theme={resolvedTheme}>
	<Editor
		onReady={onEditorReady}
		theme={resolvedTheme}
		{fontSize}
		{fontFamily}
	/>

	{#if ctxMenu.show}
		<ContextMenu contextMenu={ctxMenu} onhide={hideMenu} />
	{/if}

	{#if showSettings}
		<SettingsModal
			settings={{ theme, fontSize, fontFamily }}
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
	}
	:global(body[data-theme="light"]) {
		background: #ffffff;
		color: #333333;
	}
	:global(.cm-editor) {
		height: 100vh;
	}
</style>