<script lang="ts">
	import Nav from './components/Nav.svelte';
	import OrdenesView from './views/OrdenesView.svelte';
	import DetalleView from './views/DetalleView.svelte';
	import ConfigView from './views/ConfigView.svelte';
	import SetupView from './views/SetupView.svelte';
	import FraccionesView from './views/FraccionesView.svelte';
	import { initClient } from './lib/grpc.js';
	import { check } from '@tauri-apps/plugin-updater';

	type View = 'ordenes' | 'detalle' | 'config' | 'fracciones';

	// null = verificando actualización/configuración al inicio
	let activeView = $state<View | null>(null);
	let detalleTipodoc = $state('');
	let detalleNumdoc = $state('');
	let needsSetup = $state(false);
	let updating = $state(false);

	$effect(() => {
		(async () => {
			// Verificar e instalar actualización antes de cargar la app
			try {
				const update = await check();
				if (update?.available) {
					updating = true;
					await update.downloadAndInstall();
					// downloadAndInstall() reinicia la app automáticamente
					return;
				}
			} catch {
				// Sin conexión o error: continuar normalmente
			}

			const configured = await initClient();
			needsSetup = !configured;
			activeView = 'ordenes';
		})();
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

{#if updating}
	<div class="min-h-screen bg-bg flex items-center justify-center">
		<p class="text-text-muted text-sm">Instalando actualización…</p>
	</div>

{:else if activeView === null}
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
