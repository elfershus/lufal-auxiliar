<script lang="ts">
	import { onMount } from 'svelte';
	import SkeletonList from '../components/SkeletonList.svelte';
	import ErrorBanner from '../components/ErrorBanner.svelte';
	import { nav } from '../lib/nav.svelte.js';
	import { listDocumentos, getProveedorNombre, getDocumento, buscarSeguimiento } from '../lib/grpc.js';
	import { formatFecha, formatFechaConDia, formatFechaCorta, formatMXN, tipoColor, fechaMenos } from '../lib/utils.js';
	import { appConfig } from '../lib/config.svelte.js';
	import type { DocumentoRecord, ListDocumentosParams, MovimientoRecord, SeguimientoResult } from '../lib/types.js';

	interface Props {
		onSelectDoc: (tipodoc: string, numdoc: string) => void;
		onGoConfig: () => void;
	}
	let { onSelectDoc, onGoConfig }: Props = $props();

	// ── Estado
	let query = $state('');
	let fechaDia = $state('');
	let fechaInputEl: HTMLInputElement | undefined = $state(undefined);

	let documentos = $state<DocumentoRecord[]>([]);
	let nextPageToken = $state('');
	let nombres = $state<Record<string, string>>({});
	let movimientosPorDoc = $state<Record<string, MovimientoRecord[]>>({});
	let seguimientoPorDoc = $state<Record<string, SeguimientoResult>>({});

	let cargando = $state(false);
	let cargandoMas = $state(false);
	let cargandoArts = $state(false);
	let errorMsg = $state('');
	let mounted = false;

	const filtrosActivos = $derived((() => {
		const chips: Array<{ label: string; clear: () => void }> = [];
		if (query.trim()) chips.push({ label: `"${query.trim()}"`, clear: () => { query = ''; } });
		return chips;
	})());

	onMount(() => {
		mounted = true;
		if (appConfig.numalm) buscar();
	});

	// ── Buscar cuando cambia el texto (debounce 300ms)
	$effect(() => {
		void query;
		if (!mounted) return;
		const t = setTimeout(buscar, 300);
		return () => clearTimeout(t);
	});

	async function buscar() {
		if (!appConfig.numalm) return;
		cargando = true;
		errorMsg = '';
		try {
			const params: ListDocumentosParams = {
				numalm: appConfig.numalm,
				page_size: 10,
				order_by: 'fechacapt_desc',
			};
			params.tipodoc = 'O';
			if (query.trim()) params.numdoc = query.trim();
			if (fechaDia) params.fechacapt_from = fechaDia;
			if (fechaDia) params.fechacapt_to = fechaDia;
			const result = await listDocumentos(params);
			documentos = result.documentos;
			nextPageToken = result.next_page_token;
			nombres = {};
			movimientosPorDoc = {};
			seguimientoPorDoc = {};
			await resolverNombres(result.documentos);
			resolverArticulos(result.documentos);
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		} finally {
			cargando = false;
		}
	}

	async function cargarMas() {
		if (!nextPageToken || cargandoMas || !appConfig.numalm) return;
		cargandoMas = true;
		try {
			const params: ListDocumentosParams = {
				numalm: appConfig.numalm,
				page_size: 10,
				page_token: nextPageToken,
				order_by: 'fechacapt_desc',
			};
			params.tipodoc = 'O';
			if (query.trim()) params.numdoc = query.trim();
			if (fechaDia) params.fechacapt_from = fechaDia;
			if (fechaDia) params.fechacapt_to = fechaDia;
			const result = await listDocumentos(params);
			documentos = [...documentos, ...result.documentos];
			nextPageToken = result.next_page_token;
			await resolverNombres(result.documentos);
			resolverArticulos(result.documentos);
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		} finally {
			cargandoMas = false;
		}
	}

	async function resolverNombres(docs: DocumentoRecord[]) {
		const provs = [...new Set(docs.map((d) => d.numprov).filter(Boolean))];
		await Promise.allSettled(
			provs.map(async (p) => {
				if (!nombres[p]) {
					const n = await getProveedorNombre(p);
					if (n) nombres = { ...nombres, [p]: n };
				}
			})
		);
	}

	async function resolverArticulos(docs: DocumentoRecord[]) {
		cargandoArts = true;

		// Stage 1: movimientos (todos, para que seguimiento tenga todos los numarts)
		await Promise.allSettled(
			docs.map(async (doc) => {
				const key = doc.tipodoc + doc.numdoc;
				if (movimientosPorDoc[key]) return;
				const result = await getDocumento(doc.tipodoc, doc.numdoc);
				movimientosPorDoc = { ...movimientosPorDoc, [key]: result.movimientos };
			})
		);

		// Stage 2: seguimiento usando todos los numarts de cada doc
		await Promise.allSettled(
			docs.map(async (doc) => {
				const key = doc.tipodoc + doc.numdoc;
				if (seguimientoPorDoc[key]) return;
				const movs = movimientosPorDoc[key];
				if (!movs?.length) return;
				const numarts = [...new Set(movs.map((m) => m.numart).filter(Boolean))];
				const result = await buscarSeguimiento(numarts, fechaMenos(doc.fechacapt ?? '', 1), appConfig.numalm);
				seguimientoPorDoc = { ...seguimientoPorDoc, [key]: result };
			})
		);

		cargandoArts = false;
	}
</script>

<div class="min-h-screen bg-bg">
	<!-- Header -->
	<div class="bg-navy px-4 pt-5 pb-4 md:px-6">
		<div class="flex items-center justify-between gap-3 mb-4">
			<div class="flex items-center gap-3">
				<button
					class="md:hidden w-9 h-9 flex items-center justify-center rounded-lg text-white/70
						hover:bg-white/10 active:bg-white/20 transition-colors"
					onclick={() => nav.toggle()}
					aria-label="Abrir menú"
				>
					<svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
						<line x1="3" y1="6" x2="21" y2="6" />
						<line x1="3" y1="12" x2="21" y2="12" />
						<line x1="3" y1="18" x2="21" y2="18" />
					</svg>
				</button>
				<div>
					<p class="text-[11px] font-semibold tracking-[0.12em] uppercase text-white/40">Compras</p>
					<h1 class="font-barlow-condensed text-[22px] font-bold text-white leading-none">
						Órdenes de Compra
					</h1>
				</div>
			</div>
			<div class="flex items-center gap-2">
				<button
					onclick={buscar}
					disabled={cargando}
					class="w-8 h-8 flex items-center justify-center rounded-lg text-white/70
						border border-white/20 hover:bg-white/10 active:bg-white/20
						transition-colors disabled:opacity-40"
					aria-label="Actualizar"
				>
					<svg class="w-4 h-4 {cargando ? 'animate-spin' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
						<path d="M21 2v6h-6" />
						<path d="M3 12a9 9 0 0115-6.7L21 8" />
						<path d="M3 22v-6h6" />
						<path d="M21 12a9 9 0 01-15 6.7L3 16" />
					</svg>
				</button>
				<span class="flex items-center justify-center min-w-[40px] h-[26px] px-[10px]
					bg-white/[0.12] border border-white/20 rounded-full
					font-mono text-[13px] font-medium text-white/85">
					{#if cargando}
						<span class="w-2 h-2 rounded-full bg-amber animate-pulse-dot"></span>
					{:else}
						{documentos.length}
					{/if}
				</span>
			</div>
		</div>

		<!-- Búsqueda + Fecha en una sola fila -->
		<div class="flex gap-2">
			<div class="relative flex-1">
				<svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-white/40"
					viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
					<circle cx="11" cy="11" r="8" />
					<line x1="21" y1="21" x2="16.65" y2="16.65" />
				</svg>
				<input
					type="text"
					placeholder="Buscar por número..."
					bind:value={query}
					class="w-full h-10 pl-10 pr-3 rounded-lg text-[14px] font-barlow
						bg-white/10 text-white placeholder:text-white/40 border border-white/20
						focus:outline-none focus:ring-2 focus:ring-amber/60"
				/>
			</div>
			<div class="flex items-center h-10 rounded-lg border border-white/20 bg-white/10 overflow-hidden whitespace-nowrap">
				<button
					type="button"
					onclick={() => fechaInputEl?.showPicker()}
					class="flex items-center gap-1.5 px-3 h-full hover:bg-white/15 transition-colors focus:outline-none"
				>
					<svg class="w-4 h-4 shrink-0 {fechaDia ? 'text-amber' : 'text-white/40'}"
						viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
						<rect x="3" y="4" width="18" height="18" rx="2" />
						<line x1="16" y1="2" x2="16" y2="6" />
						<line x1="8" y1="2" x2="8" y2="6" />
						<line x1="3" y1="10" x2="21" y2="10" />
					</svg>
					<span class="text-[12px] {fechaDia ? 'text-white' : 'text-white/40'}">
						{fechaDia ? formatFechaCorta(fechaDia) : 'Fecha'}
					</span>
				</button>
				{#if fechaDia}
					<button
						type="button"
						onclick={() => { fechaDia = ''; buscar(); }}
						class="pr-2.5 h-full flex items-center text-white/40 hover:text-white transition-colors focus:outline-none"
						aria-label="Limpiar fecha"
					>
						<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
							<line x1="18" y1="6" x2="6" y2="18" />
							<line x1="6" y1="6" x2="18" y2="18" />
						</svg>
					</button>
				{/if}
				<input
					bind:this={fechaInputEl}
					type="date"
					value={fechaDia}
					onchange={(e) => { fechaDia = e.currentTarget.value; buscar(); }}
					class="sr-only"
				/>
			</div>
		</div>

	</div>

	<!-- Chips de filtros activos -->
	{#if filtrosActivos.length > 0}
		<div class="px-4 pt-2 flex gap-2 flex-wrap animate-fadeDown">
			{#each filtrosActivos as f (f.label)}
				<button
					class="flex items-center gap-1 px-2.5 py-0.5 rounded-full
						bg-amber/15 text-amber text-[11px] font-medium
						hover:bg-amber/25 transition-colors"
					onclick={f.clear}
				>
					{f.label}
					<svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
						<line x1="18" y1="6" x2="6" y2="18" />
						<line x1="6" y1="6" x2="18" y2="18" />
					</svg>
				</button>
			{/each}
		</div>
	{/if}

	<!-- Cuerpo -->
	<div class="px-4 py-4 md:px-6">
		{#if !appConfig.numalm}
			<div class="flex flex-col items-center justify-center py-16 text-center animate-fadeSlide">
				<div class="w-12 h-12 rounded-full bg-slate-100 flex items-center justify-center mb-3">
					<svg class="w-6 h-6 text-slate-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="12" cy="12" r="3" />
						<path d="M19.07 4.93a10 10 0 010 14.14M4.93 4.93a10 10 0 000 14.14" />
					</svg>
				</div>
				<p class="text-[14px] font-medium text-slate-500">Almacén no configurado</p>
				<p class="text-[12px] text-slate-400 mt-1 mb-4">Configura el almacén antes de continuar</p>
				<button
					onclick={onGoConfig}
					class="px-4 py-2 rounded-lg bg-navy text-white text-[13px] font-medium font-barlow
						hover:bg-navy-light transition-colors"
				>
					Ir a Configuración
				</button>
			</div>
		{:else if cargando}
			<SkeletonList count={5} />
		{:else if errorMsg}
			<ErrorBanner message={errorMsg} onRetry={buscar} />
		{:else if documentos.length === 0}
			<div class="flex flex-col items-center justify-center py-16 text-center animate-fadeSlide">
				<div class="w-12 h-12 rounded-full bg-slate-100 flex items-center justify-center mb-3">
					<svg class="w-6 h-6 text-slate-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">
						<path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
						<polyline points="14 2 14 8 20 8" />
					</svg>
				</div>
				<p class="text-[14px] font-medium text-slate-500">Sin documentos</p>
				<p class="text-[12px] text-slate-400 mt-1">No se encontraron órdenes con los filtros actuales</p>
			</div>
		{:else}
			<div class="flex flex-col gap-2">
				{#each documentos as doc (doc.tipodoc + doc.numdoc)}
					{@const color = tipoColor(doc.tipodoc)}
					{@const docKey = doc.tipodoc + doc.numdoc}
					{@const movs = movimientosPorDoc[docKey]}
					{@const seg = seguimientoPorDoc[docKey]}
					<button
						class="w-full bg-surface rounded-card py-2.5 px-4 shadow-card text-left
							hover:shadow-card-md active:scale-[0.99] transition-all duration-150 animate-fadeSlide"
						style="border-left: 3.5px solid {color}"
						onclick={() => onSelectDoc(doc.tipodoc, doc.numdoc)}
					>
						<div class="flex items-center justify-between gap-2 mb-1">
							<div class="flex items-center gap-2 min-w-0">
								<span
									class="shrink-0 px-2 py-0.5 rounded-md text-[12px] font-mono text-white"
									style="background-color: {color}"
								>
									{doc.numdoc.trim()}
								</span>
								<p class="text-[13px] font-medium text-slate-700 truncate">
									{(nombres[doc.numprov] ?? doc.numprov) || '—'}
								</p>
							</div>
							{#if doc.fechacapt}
								<span class="shrink-0 text-[12px] font-semibold text-slate-500">{formatFechaConDia(doc.fechacapt)}</span>
							{/if}
						</div>

						{#if doc.refer?.trim()}
							<div class="mt-0.5 text-[11px] text-slate-400 truncate">
								{doc.refer.trim()}
							</div>
						{/if}

						{#if movs || cargandoArts}
							<div class="mt-1.5 flex items-start justify-between gap-2">
								<!-- Artículos: izquierda -->
								<div class="flex flex-wrap gap-1 flex-1">
									{#if movs && movs.length > 0}
										{#each movs.slice(0, 5) as m (m.numpar)}
											<span class="px-1.5 py-0.5 rounded bg-slate-100
												font-mono text-[10px] text-slate-500 leading-none">
												{m.numart}
											</span>
										{/each}
									{:else}
										{#each [1, 2, 3] as _}
											<span class="inline-block h-4 w-16 rounded bg-slate-100 animate-pulse"></span>
										{/each}
									{/if}
								</div>
								<!-- Seguimiento: derecha -->
								<div class="flex flex-wrap gap-1 justify-end shrink-0">
									{#if seg}
										{#each seg.compras as c (c.numdoc)}
											<span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded font-mono text-[10px] text-white bg-cyan-600 leading-none">
												<span>{c.tipodoc}</span>
												<span>{c.numdoc.trim()}</span>
												<span class="opacity-75">{Math.round(c.coverage_pct)}%</span>
											</span>
										{/each}
										{#each seg.remisiones as r (r.numdoc)}
											<span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded font-mono text-[10px] text-white bg-blue-600 leading-none">
												<span>{r.tipodoc}</span>
												<span>{r.numdoc.trim()}</span>
												<span class="opacity-75">{Math.round(r.coverage_pct)}%</span>
											</span>
										{/each}
									{:else if movs}
										<!-- movimientos cargados pero seguimiento aún en camino -->
										{#each [1, 2] as _}
											<span class="inline-block h-4 w-10 rounded bg-slate-100 animate-pulse"></span>
										{/each}
									{/if}
								</div>
							</div>
						{/if}
					</button>
				{/each}
			</div>

			<!-- Cargar más -->
			{#if nextPageToken}
				<div class="mt-4 text-center">
					<button
						onclick={cargarMas}
						disabled={cargandoMas}
						class="px-6 py-2 rounded-lg bg-navy text-white text-[13px] font-medium font-barlow
							hover:bg-navy-light active:bg-navy-dark transition-colors
							disabled:opacity-50 disabled:cursor-not-allowed"
					>
						{#if cargandoMas}
							<span class="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin-fast mr-2"></span>
						{/if}
						Cargar más
					</button>
				</div>
			{/if}
		{/if}
	</div>
</div>
