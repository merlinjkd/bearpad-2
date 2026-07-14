<script lang="ts">
	import { onMount } from 'svelte';
	import Editor from './lib/Editor.svelte';
	import ContextMenu from './lib/ContextMenu.svelte';

	let editorRef = $state<{
		hasSelection: () => boolean;
		handleCut: () => Promise<void>;
		handleCopy: () => Promise<void>;
		handlePaste: () => Promise<void>;
		undo: () => void;
		redo: () => void;
		handleSelectAll: () => void;
		transformSelection: (type: 'lowercase' | 'uppercase' | 'propercase') => void;
	} | null>(null);

	let contextMenu = $state<{ show: boolean; x: number; y: number; items: any[] }>({
		show: false,
		x: 0,
		y: 0,
		items: []
	});

	let editorEl: HTMLDivElement;

	function onContextMenu(e: MouseEvent) {
		e.preventDefault();
		const isInsideEditor = !!(e.target as HTMLElement).closest('.cm-editor');
		const selection = editorRef?.hasSelection();

		const items = [];

		if (isInsideEditor) {
			if (selection) {
				items.push(
					{ label: 'Cut', shortcut: '⌘X', onClick: () => editorRef?.handleCut() },
					{ label: 'Copy', shortcut: '⌘C', onClick: () => editorRef?.handleCopy() }
				);
			}
			items.push(
				{ label: 'Paste', shortcut: '⌘V', onClick: () => editorRef?.handlePaste() },
				{ separator: true },
				{ label: 'Undo', shortcut: '⌘Z', onClick: () => editorRef?.undo() },
				{ label: 'Redo', shortcut: '⌘Y', onClick: () => editorRef?.redo() },
				{ separator: true },
				{ label: 'lowercase', disabled: !selection, onClick: () => { editorRef?.transformSelection('lowercase'); contextMenu.show = false; } },
				{ label: 'UPPERCASE', disabled: !selection, onClick: () => { editorRef?.transformSelection('uppercase'); contextMenu.show = false; } },
				{ label: 'Title Case', disabled: !selection, onClick: () => { editorRef?.transformSelection('propercase'); contextMenu.show = false; } },
				{ separator: true }
			);
		}

		items.push(
			{ label: 'Select All', shortcut: '⌘A', onClick: () => editorRef?.handleSelectAll() }
		);

		contextMenu = { show: true, x: e.clientX, y: e.clientY, items };
	}

	function hideMenu() {
		contextMenu.show = false;
	}

	onMount(() => {
		document.addEventListener('contextmenu', onContextMenu);
		document.addEventListener('click', (e) => {
			if (contextMenu.show && !(e.target as HTMLElement).closest('.custom-context-menu')) {
				hideMenu();
			}
		});
	});
</script>

<div bind:this={editorEl}>
	<Editor bind:editorRef />
</div>

{#if contextMenu.show}
	<ContextMenu {contextMenu} onhide={hideMenu} />
{/if}

<style>
	:global(body) {
		margin: 0;
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
		background: #1e1e1e;
		color: #d4d4d4;
	}
	:global(.cm-editor) {
		height: 100vh;
	}
	:global(.cm-scroller) {
		font-family: 'SF Mono', 'Fira Code', 'Fira Code VF', 'Cascadia Code', monospace;
		font-size: 14px;
	}
</style>
