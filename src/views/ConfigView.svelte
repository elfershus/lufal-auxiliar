<script lang="ts">
	import { open } from '@tauri-apps/plugin-dialog';
	import { listAlmacenes } from '../lib/grpc.js';
	import { getSucursalesConfig, saveSucursalDbfPath, saveDefaultNumalm, saveSucursalesMap } from '../lib/dbf.js';
	import { appConfig } from '../lib/config.svelte.js';
	import type { AlmacenRecord, SucursalEntry, SucursalesConfig } from '../lib/types.js';

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

	// ── Almacenes y sucursales
	let almacenes = $state<AlmacenRecord[]>([]);
	let sucursalesConfig = $state<SucursalesConfig>({ sucursales: [], default_numalm: null });
	let cargando = $state(false);
	let errorMsg = $state('');
	let savingDbfFor = $state<string | null>(null);
	let dbfError = $state('');

	$effect(() => {
		if (unlocked) {
			cargar();
		}
	});

	async function cargar() {
		cargando = true;
		errorMsg = '';
		try {
			const [alms, cfg] = await Promise.all([listAlmacenes(), getSucursalesConfig()]);
			almacenes = alms;
			sucursalesConfig = cfg;

			// Auto-sincronizar letras desde la API
			const entries: SucursalEntry[] = alms.map((alm) => {
				const existing = cfg.sucursales.find((s) => s.numalm === alm.numalm);
				return { numalm: alm.numalm, letra: alm.letra, dbf_path: existing?.dbf_path ?? null };
			});
			const validas = entries.filter((s) => s.letra.trim().length > 0);
			if (validas.length > 0) await saveSucursalesMap(validas);
			// Reflejar dbf_paths actualizados en el estado local
			sucursalesConfig = { ...sucursalesConfig, sucursales: entries };
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		} finally {
			cargando = false;
		}
	}

	async function asignarCarpeta(numalm: string) {
		dbfError = '';
		const selected = await open({ directory: true, multiple: false, title: 'Seleccionar carpeta de archivos DBF' });
		if (!selected || typeof selected !== 'string') return;
		savingDbfFor = numalm;
		try {
			await saveSucursalDbfPath(numalm, selected);
			sucursalesConfig = {
				...sucursalesConfig,
				sucursales: sucursalesConfig.sucursales.map((s) =>
					s.numalm === numalm ? { ...s, dbf_path: selected } : s
				)
			};
		} catch (e) {
			dbfError = e instanceof Error ? e.message : String(e);
		} finally {
			savingDbfFor = null;
		}
	}

	async function setDefault(numalm: string) {
		try {
			await saveDefaultNumalm(numalm);
			sucursalesConfig = { ...sucursalesConfig, default_numalm: numalm };
			appConfig.numalm = numalm;
			const alm = almacenes.find((a) => a.numalm === numalm);
			if (alm) appConfig.nomalm = alm.nomalm;
		} catch (e) {
			dbfError = e instanceof Error ? e.message : String(e);
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
			<!-- Sucursales -->
			<div class="bg-surface rounded-card p-5 shadow-card animate-fadeSlide">
				<h2 class="text-[11px] font-semibold tracking-[0.1em] uppercase text-slate-400 mb-1">
					Sucursales
				</h2>
				<p class="text-[11px] text-slate-400 mb-4">
					Asigna la carpeta DBF de cada sucursal y marca cuál abre al iniciar la app.
				</p>

				{#if dbfError}
					<p class="text-[12px] text-red-500 mb-3">{dbfError}</p>
				{/if}

				{#if cargando}
					<div class="space-y-2">
						{#each [1, 2] as _}
							<div class="h-14 rounded-lg animate-shimmer"
								style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
							</div>
						{/each}
					</div>
				{:else if errorMsg}
					<p class="text-[13px] text-red-500">{errorMsg}</p>
				{:else}
					<div class="border border-slate-200 rounded-lg overflow-hidden">
						<!-- Encabezado -->
						<div class="grid grid-cols-[3.5rem_1fr_auto_auto] bg-bg border-b border-slate-200 px-3 py-1.5 gap-2">
							<span class="text-[10px] font-mono font-semibold text-slate-400 uppercase tracking-wider">Núm.</span>
							<span class="text-[10px] font-mono font-semibold text-slate-400 uppercase tracking-wider">Nombre / Carpeta DBF</span>
							<span class="text-[10px] font-mono font-semibold text-slate-400 uppercase tracking-wider text-center w-16">Carpeta</span>
							<span class="text-[10px] font-mono font-semibold text-slate-400 uppercase tracking-wider text-center w-10">Default</span>
						</div>

						{#each almacenes as alm, i (alm.numalm)}
							{@const entry = sucursalesConfig.sucursales.find((s) => s.numalm === alm.numalm)}
							{@const isDefault = sucursalesConfig.default_numalm === alm.numalm}
							{@const isSaving = savingDbfFor === alm.numalm}
							<div class="grid grid-cols-[3.5rem_1fr_auto_auto] px-3 py-2 items-center gap-2 {i > 0 ? 'border-t border-slate-100' : ''}">
								<!-- Núm -->
								<span class="font-mono text-[12px] text-slate-500">{alm.numalm}</span>

								<!-- Nombre + carpeta -->
								<div class="min-w-0">
									<p class="text-[12px] text-slate-700 font-medium truncate">{alm.nomalm}</p>
									{#if entry?.dbf_path}
										<p class="font-mono text-[10px] text-slate-400 truncate" title={entry.dbf_path}>
											{entry.dbf_path}
										</p>
									{:else}
										<p class="text-[10px] text-slate-300 italic">Sin carpeta asignada</p>
									{/if}
								</div>

								<!-- Botón carpeta -->
								<button
									onclick={() => asignarCarpeta(alm.numalm)}
									disabled={isSaving}
									title={entry?.dbf_path ? 'Cambiar carpeta DBF' : 'Asignar carpeta DBF'}
									class="w-16 h-7 rounded text-[11px] font-medium font-barlow transition-colors flex items-center justify-center gap-1
										{entry?.dbf_path
											? 'bg-bg border border-slate-200 text-slate-500 hover:bg-slate-50'
											: 'bg-navy text-white hover:opacity-90'}
										disabled:opacity-40"
								>
									{#if isSaving}
										<svg class="w-3 h-3 animate-spin-fast" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
											<path d="M21 12a9 9 0 11-6.219-8.56" />
										</svg>
									{:else}
										<svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
											<path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
										</svg>
										{entry?.dbf_path ? 'Cambiar' : 'Asignar'}
									{/if}
								</button>

								<!-- Radio predeterminado -->
								<div class="flex justify-center w-10">
									<button
										onclick={() => setDefault(alm.numalm)}
										title="Marcar como predeterminado"
										class="w-5 h-5 rounded-full border-2 flex items-center justify-center transition-colors
											{isDefault
												? 'border-amber bg-amber'
												: 'border-slate-300 bg-bg hover:border-amber/60'}"
									>
										{#if isDefault}
											<div class="w-2 h-2 rounded-full bg-white"></div>
										{/if}
									</button>
								</div>
							</div>
						{/each}
					</div>

					{#if sucursalesConfig.default_numalm}
						<p class="text-[11px] text-slate-400 mt-2">
							Predeterminado: <span class="font-mono font-semibold text-slate-600">{sucursalesConfig.default_numalm}</span>
							— {almacenes.find((a) => a.numalm === sucursalesConfig.default_numalm)?.nomalm ?? ''}
						</p>
					{/if}
				{/if}
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
