<script lang="ts">
	import { onMount } from 'svelte';
	import Nav from './components/Nav.svelte';
	import OrdenesView from './views/OrdenesView.svelte';
	import DetalleView from './views/DetalleView.svelte';
	import ConfigView from './views/ConfigView.svelte';
	import SetupView from './views/SetupView.svelte';
	import FraccionesView from './views/FraccionesView.svelte';
	import { initClient } from './lib/grpc.js';
	import { check, type Update } from '@tauri-apps/plugin-updater';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	type View = 'ordenes' | 'detalle' | 'config' | 'fracciones';

	// null = verificando configuración al inicio
	let activeView = $state<View | null>(null);
	let detalleTipodoc = $state('');
	let detalleNumdoc = $state('');
	let needsSetup = $state(false);

	let pendingUpdate = $state<Update | null>(null);
	let updateDownloaded = $state(false);

	$effect(() => {
		initClient().then((configured) => {
			needsSetup = !configured;
			activeView = 'ordenes';
		});

		// Verificar actualización en background; descargar si hay una disponible
		check().then(async (update) => {
			if (update?.available) {
				pendingUpdate = update;
				await update.download();
				updateDownloaded = true;
			}
		}).catch(() => {});
	});

	onMount(() => {
		const appWindow = getCurrentWindow();
		let unlisten: (() => void) | undefined;

		// Instalar silenciosamente al cerrar si la descarga ya terminó
		appWindow.onCloseRequested(async (event) => {
			if (pendingUpdate && updateDownloaded) {
				event.preventDefault();
				try {
					await pendingUpdate.install();
				} catch {
					appWindow.destroy();
				}
			}
		}).then(fn => { unlisten = fn; });

		return () => { unlisten?.(); };
	});

	function goToDetalle(tipodoc: string, numdoc: string) {
		detalleTipodoc = tipodoc;
		detalleNumdoc = numdoc;
		activeView = 'detalle';
	}

	function goToList() {
		activeView = 'ordenes';
	}

	function navigate(view: string) {
		activeView = view as View;
	}
</script>

{#if activeView === null}
	<!-- Verificando configuración inicial -->
	<div class="min-h-screen bg-bg"></div>

{:else if needsSetup}
	<SetupView onDone={() => { needsSetup = false; }} />

{:else}
	<Nav {activeView} onNavigate={navigate} />
	<main class="md:pl-52 min-h-screen">
		{#if activeView === 'ordenes'}
			<OrdenesView onSelectDoc={goToDetalle} onGoConfig={() => navigate('config')} />
		{:else if activeView === 'detalle'}
			<DetalleView
				tipodoc={detalleTipodoc}
				numdoc={detalleNumdoc}
				onBack={goToList}
			/>
		{:else if activeView === 'config'}
			<ConfigView onBack={goToList} />
		{:else if activeView === 'fracciones'}
			<FraccionesView onGoConfig={() => navigate('config')} />
		{/if}
	</main>
{/if}
