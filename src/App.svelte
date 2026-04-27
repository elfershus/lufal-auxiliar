<script lang="ts">
	import Nav from './components/Nav.svelte';
	import OrdenesView from './views/OrdenesView.svelte';
	import DetalleView from './views/DetalleView.svelte';
	import ConfigView from './views/ConfigView.svelte';
	import SetupView from './views/SetupView.svelte';
	import FraccionesView from './views/FraccionesView.svelte';
	import UpdateBanner from './components/UpdateBanner.svelte';
	import { initClient } from './lib/grpc.js';
	import { checkForUpdates, type Update } from './lib/updater.js';

	type View = 'ordenes' | 'detalle' | 'config' | 'fracciones';

	// null = verificando configuración al inicio
	let activeView = $state<View | null>(null);
	let detalleTipodoc = $state('');
	let detalleNumdoc = $state('');
	let needsSetup = $state(false);
	let pendingUpdate = $state<Update | null>(null);

	$effect(() => {
		initClient().then((configured) => {
			needsSetup = !configured;
			activeView = 'ordenes';
		});

		// Revisar actualizaciones en segundo plano, sin bloquear el inicio
		checkForUpdates().then((update) => {
			if (update?.available) pendingUpdate = update;
		});
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

{#if pendingUpdate}
	<UpdateBanner update={pendingUpdate} onDismiss={() => (pendingUpdate = null)} />
{/if}
