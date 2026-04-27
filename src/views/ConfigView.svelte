<script lang="ts">
	import { open } from '@tauri-apps/plugin-dialog';
	import { listAlmacenes } from '../lib/grpc.js';
	import { getDbfPaths, saveDbfArts, saveDbfUnidades } from '../lib/dbf.js';
	import { appConfig } from '../lib/config.svelte.js';
	import type { AlmacenRecord } from '../lib/types.js';

	const PASSWORD = 'wombocombo69';

	interface Props {
		onBack: () => void;
	}
	let { onBack }: Props = $props();

	// ── Auth gate
	let unlocked = $state(false);
	let passwordInput = $state('');
	let passwordError = $state(false);
	let shake = $state(false);

	function submitPassword() {
		if (passwordInput === PASSWORD) {
			unlocked = true;
			passwordError = false;
		} else {
			passwordError = true;
			passwordInput = '';
			shake = true;
			setTimeout(() => (shake = false), 500);
		}
	}

	// ── Almacén config
	let almacenes = $state<AlmacenRecord[]>([]);
	let selectedNumalm = $state(appConfig.numalm);
	let cargando = $state(false);
	let errorMsg = $state('');
	let saved = $state(false);

	// ── DBF paths config
	let artsPath = $state('');
	let unidadesPath = $state('');
	let savingArts = $state(false);
	let savingUnidades = $state(false);
	let savedArts = $state(false);
	let savedUnidades = $state(false);
	let dbfError = $state('');

	$effect(() => {
		if (unlocked) {
			cargarAlmacenes();
			getDbfPaths().then((p) => {
				artsPath = p.dbf_arts ?? '';
				unidadesPath = p.dbf_unidades ?? '';
			});
		}
	});

	async function cargarAlmacenes() {
		cargando = true;
		errorMsg = '';
		try {
			almacenes = await listAlmacenes();
			if (!selectedNumalm && almacenes.length > 0) {
				selectedNumalm = almacenes[0].numalm;
			}
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		} finally {
			cargando = false;
		}
	}

	function guardar() {
		appConfig.numalm = selectedNumalm;
		const alm = almacenes.find((a) => a.numalm === selectedNumalm);
		appConfig.nomalm = alm?.nomalm ?? '';
		saved = true;
		setTimeout(() => onBack(), 800);
	}

	async function seleccionarArts() {
		dbfError = '';
		const selected = await open({
			directory: false,
			multiple: false,
			title: 'Seleccionar archivo de artículos (.DBF)',
			filters: [{ name: 'dBASE', extensions: ['dbf', 'DBF'] }]
		});
		if (!selected || typeof selected !== 'string') return;
		savingArts = true;
		try {
			await saveDbfArts(selected);
			artsPath = selected;
			savedArts = true;
			setTimeout(() => (savedArts = false), 2000);
		} catch (e) {
			dbfError = e instanceof Error ? e.message : String(e);
		} finally {
			savingArts = false;
		}
	}

	async function seleccionarUnidades() {
		dbfError = '';
		const selected = await open({
			directory: false,
			multiple: false,
			title: 'Seleccionar archivo de fracciones (.DBF)',
			filters: [{ name: 'dBASE', extensions: ['dbf', 'DBF'] }]
		});
		if (!selected || typeof selected !== 'string') return;
		savingUnidades = true;
		try {
			await saveDbfUnidades(selected);
			unidadesPath = selected;
			savedUnidades = true;
			setTimeout(() => (savedUnidades = false), 2000);
		} catch (e) {
			dbfError = e instanceof Error ? e.message : String(e);
		} finally {
			savingUnidades = false;
		}
	}
</script>

<div class="min-h-screen bg-bg">
	<!-- Header -->
	<div class="bg-[#0f1f38] px-4 pt-5 pb-4 md:px-6 flex items-center gap-3">
		<button
			onclick={onBack}
			class="w-9 h-9 flex items-center justify-center rounded-lg text-white/70
				hover:bg-white/10 active:bg-white/20 transition-colors"
			aria-label="Regresar"
		>
			<svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
				<path d="M19 12H5M12 5l-7 7 7 7" />
			</svg>
		</button>
		<div>
			<p class="text-[11px] font-semibold tracking-[0.12em] uppercase text-white/40">Sistema</p>
			<h1 class="font-barlow-condensed text-[22px] font-bold text-white leading-none">Configuración</h1>
		</div>
	</div>

	<div class="px-4 py-6 md:px-6 max-w-md">
		{#if !unlocked}
			<!-- Password gate -->
			<div class="bg-surface rounded-card p-6 shadow-card animate-fadeSlide {shake ? 'animate-shake' : ''}">
				<div class="flex flex-col items-center mb-5">
					<div class="w-12 h-12 rounded-full bg-navy/10 flex items-center justify-center mb-3">
						<svg class="w-6 h-6 text-navy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
							<rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
							<path d="M7 11V7a5 5 0 0110 0v4" />
						</svg>
					</div>
					<h2 class="font-barlow-condensed text-[18px] font-bold text-navy">Acceso restringido</h2>
					<p class="text-[12px] text-slate-400 mt-1 text-center">Ingresa la contraseña para acceder a la configuración</p>
				</div>

				<form onsubmit={(e) => { e.preventDefault(); submitPassword(); }}>
					<input
						type="password"
						placeholder="Contraseña"
						bind:value={passwordInput}
						class="w-full h-10 px-3 rounded-lg text-[14px] font-barlow mb-3
							bg-bg border {passwordError ? 'border-red-300' : 'border-slate-200'}
							focus:outline-none focus:ring-2 {passwordError ? 'focus:ring-red-300' : 'focus:ring-amber/60'}
							text-slate-700 placeholder:text-slate-400 transition-colors"
					/>
					{#if passwordError}
						<p class="text-[12px] text-red-500 mb-3 text-center">Contraseña incorrecta</p>
					{/if}
					<button
						type="submit"
						class="w-full h-10 rounded-lg bg-navy text-white text-[14px] font-medium font-barlow
							hover:bg-navy-light active:bg-navy-dark transition-colors"
					>
						Entrar
					</button>
				</form>
			</div>

		{:else}
			<!-- Almacén config -->
			<div class="bg-surface rounded-card p-5 shadow-card animate-fadeSlide">
				<h2 class="text-[11px] font-semibold tracking-[0.1em] uppercase text-slate-400 mb-4">
					Almacén activo
				</h2>

				{#if cargando}
					<div class="h-10 rounded-lg animate-shimmer mb-4"
						style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
					</div>
				{:else if errorMsg}
					<p class="text-[13px] text-red-500 mb-4">{errorMsg}</p>
				{:else}
					<select
						bind:value={selectedNumalm}
						class="w-full h-10 rounded-lg px-3 text-[14px] font-barlow font-medium mb-4
							bg-bg text-slate-700 border border-slate-200
							focus:outline-none focus:ring-2 focus:ring-amber/60"
					>
						{#each almacenes as alm (alm.numalm)}
							<option value={alm.numalm}>
								{alm.numalm} — {alm.nomalm}
							</option>
						{/each}
					</select>
				{/if}

				<button
					onclick={guardar}
					disabled={!selectedNumalm || cargando || saved}
					class="w-full h-10 rounded-lg text-[14px] font-medium font-barlow transition-colors
						{saved
							? 'bg-green-500 text-white cursor-default'
							: 'bg-amber text-white hover:opacity-90 active:opacity-80 disabled:opacity-40 disabled:cursor-not-allowed'}"
				>
					{saved ? '✓ Guardado' : 'Guardar'}
				</button>
			</div>

			<!-- DBF files -->
			<div class="bg-surface rounded-card p-5 shadow-card mt-4 animate-fadeSlide" style="animation-delay: 60ms">
				<h2 class="text-[11px] font-semibold tracking-[0.1em] uppercase text-slate-400 mb-1">
					Archivos DBF
				</h2>
				<p class="text-[11px] text-slate-400 mb-4">
					Selecciona cada archivo individualmente. Pueden tener cualquier nombre.
				</p>

				{#if dbfError}
					<p class="text-[12px] text-red-500 mb-3">{dbfError}</p>
				{/if}

				<!-- Artículos -->
				<div class="mb-3">
					<p class="text-[11px] font-semibold text-slate-500 mb-1.5">Archivo de artículos</p>
					{#if artsPath}
						<div class="flex items-center gap-2 mb-2 px-3 py-1.5 rounded-lg bg-bg border border-slate-200">
							<svg class="w-3 h-3 text-slate-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" /><polyline points="14 2 14 8 20 8" />
							</svg>
							<span class="font-mono text-[11px] text-slate-600 truncate flex-1" title={artsPath}>
								{artsPath.split(/[\\/]/).pop()}
							</span>
							{#if savedArts}
								<svg class="w-3 h-3 text-green-500 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
									<polyline points="20 6 9 17 4 12" />
								</svg>
							{/if}
						</div>
					{/if}
					<button
						onclick={seleccionarArts}
						disabled={savingArts}
						class="w-full h-9 rounded-lg text-[13px] font-medium font-barlow transition-colors flex items-center justify-center gap-2
							{artsPath
								? 'bg-bg border border-slate-200 text-slate-600 hover:bg-slate-50 active:bg-slate-100'
								: 'bg-navy text-white hover:opacity-90 active:opacity-80'}
							disabled:opacity-40 disabled:cursor-not-allowed"
					>
						{#if savingArts}
							<svg class="w-3.5 h-3.5 animate-spin-fast" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
								<path d="M21 12a9 9 0 11-6.219-8.56" />
							</svg>
							Guardando…
						{:else}
							<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
								<path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" /><polyline points="14 2 14 8 20 8" />
							</svg>
							{artsPath ? 'Cambiar archivo de artículos' : 'Seleccionar archivo de artículos'}
						{/if}
					</button>
				</div>

				<div class="border-t border-slate-100 my-3"></div>

				<!-- Fracciones -->
				<div>
					<p class="text-[11px] font-semibold text-slate-500 mb-1.5">Archivo de fracciones</p>
					{#if unidadesPath}
						<div class="flex items-center gap-2 mb-2 px-3 py-1.5 rounded-lg bg-bg border border-slate-200">
							<svg class="w-3 h-3 text-slate-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" /><polyline points="14 2 14 8 20 8" />
							</svg>
							<span class="font-mono text-[11px] text-slate-600 truncate flex-1" title={unidadesPath}>
								{unidadesPath.split(/[\\/]/).pop()}
							</span>
							{#if savedUnidades}
								<svg class="w-3 h-3 text-green-500 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
									<polyline points="20 6 9 17 4 12" />
								</svg>
							{/if}
						</div>
					{/if}
					<button
						onclick={seleccionarUnidades}
						disabled={savingUnidades}
						class="w-full h-9 rounded-lg text-[13px] font-medium font-barlow transition-colors flex items-center justify-center gap-2
							{unidadesPath
								? 'bg-bg border border-slate-200 text-slate-600 hover:bg-slate-50 active:bg-slate-100'
								: 'bg-navy text-white hover:opacity-90 active:opacity-80'}
							disabled:opacity-40 disabled:cursor-not-allowed"
					>
						{#if savingUnidades}
							<svg class="w-3.5 h-3.5 animate-spin-fast" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
								<path d="M21 12a9 9 0 11-6.219-8.56" />
							</svg>
							Guardando…
						{:else}
							<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
								<line x1="19" y1="5" x2="5" y2="19" /><circle cx="6.5" cy="6.5" r="2.5" /><circle cx="17.5" cy="17.5" r="2.5" />
							</svg>
							{unidadesPath ? 'Cambiar archivo de fracciones' : 'Seleccionar archivo de fracciones'}
						{/if}
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	@keyframes shake {
		0%, 100% { transform: translateX(0); }
		20%       { transform: translateX(-8px); }
		40%       { transform: translateX(8px); }
		60%       { transform: translateX(-6px); }
		80%       { transform: translateX(6px); }
	}
	.animate-shake {
		animation: shake 0.45s ease-in-out;
	}
</style>
