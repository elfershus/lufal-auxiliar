<script lang="ts">
	import { onMount } from 'svelte';
	import ErrorBanner from '../components/ErrorBanner.svelte';
	import { getDocumento, buscarSeguimiento, getArticulos } from '../lib/grpc.js';
	import { formatFecha, formatFechaConDia, formatMXN, statusInfo, tipoColor, tipoLabel, lineaImporte, fechaMenos } from '../lib/utils.js';
	import type { DocumentoRecord, MovimientoRecord, CompraMatchItem, RemisionMatchItem, ArticuloInfo } from '../lib/types.js';
	import { appConfig } from '../lib/config.svelte.js';

	interface Props {
		tipodoc: string;
		numdoc: string;
		onBack: () => void;
	}
	let { tipodoc, numdoc, onBack }: Props = $props();

	let documento = $state<DocumentoRecord | null>(null);
	let movimientos = $state<MovimientoRecord[]>([]);
	let cargando = $state(true);
	let errorMsg = $state('');

	let articulos = $state<Record<string, ArticuloInfo>>({});

	let seguimientoCompras = $state<CompraMatchItem[]>([]);
	let seguimientoRemisiones = $state<RemisionMatchItem[]>([]);
	let cargandoSeguimiento = $state(false);
	let errorSeguimiento = $state('');

	const subtotal = $derived(
		movimientos.reduce((acc, m) => acc + lineaImporte(m.cant, m.precio, m.pjedesc), 0)
	);
	const totalImpuesto = $derived(
		movimientos.reduce((acc, m) => acc + lineaImporte(m.cant, m.precio, m.pjedesc) * ((m.impuesto1 + m.impuesto2) / 100), 0)
	);

	const color = $derived(documento ? tipoColor(documento.tipodoc) : '#1e3a5f');

	onMount(async () => {
		await cargar();
		if (documento && movimientos.length > 0) {
			const numarts = [...new Set(movimientos.map((m) => m.numart).filter(Boolean))];
			cargarSeguimiento();
			getArticulos(numarts).then((result) => { articulos = result; }).catch(() => {});
		}
	});

	async function cargar() {
		cargando = true;
		errorMsg = '';
		try {
			const result = await getDocumento(tipodoc, numdoc);
			documento = result.documento;
			movimientos = result.movimientos;
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		} finally {
			cargando = false;
		}
	}

	async function cargarSeguimiento() {
		if (!documento || movimientos.length === 0) return;
		cargandoSeguimiento = true;
		errorSeguimiento = '';
		try {
			const numarts = [...new Set(movimientos.map((m) => m.numart).filter(Boolean))];
			const result = await buscarSeguimiento(numarts, fechaMenos(documento.fechacapt ?? '', 1), appConfig.numalm);
			seguimientoCompras = result.compras;
			seguimientoRemisiones = result.remisiones;
		} catch (e) {
			errorSeguimiento = e instanceof Error ? e.message : String(e);
		} finally {
			cargandoSeguimiento = false;
		}
	}
</script>

<div class="min-h-screen bg-bg">
	<!-- Header -->
	<div class="px-4 pt-5 pb-4 md:px-6" style="background-color: {color}">
		<div class="flex items-center gap-3 mb-3">
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
				{#if documento}
					<p class="text-[11px] font-semibold tracking-[0.12em] uppercase text-white/50">
						{tipoLabel(documento.tipodoc)}
					</p>
					<h1 class="font-barlow-condensed text-[22px] font-bold text-white leading-none font-mono">
						{documento.numdoc.trim()}
					</h1>
					{#if documento.fechacapt}
						<p class="text-[11px] text-white/60 mt-0.5">{formatFechaConDia(documento.fechacapt)}</p>
					{/if}
				{:else}
					<p class="text-[11px] text-white/50 uppercase tracking-wide">Cargando...</p>
				{/if}
			</div>
		</div>
	</div>

	<!-- Cuerpo -->
	<div class="px-4 py-3 md:px-6">
		{#if cargando}
			<div class="flex flex-col gap-3">
				{#each { length: 3 } as _, i (i)}
					<div class="bg-surface rounded-card p-4 shadow-card">
						<div class="h-4 w-1/2 rounded animate-shimmer mb-2"
							style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
						</div>
						<div class="h-3 w-3/4 rounded animate-shimmer"
							style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
						</div>
					</div>
				{/each}
			</div>
		{:else if errorMsg}
			<ErrorBanner message={errorMsg} onRetry={cargar} />
		{:else if documento}
			<!-- Seguimiento -->
			{#if movimientos.length > 0}
				<div class="bg-surface rounded-card shadow-card mb-3 overflow-hidden animate-fadeSlide" style="animation-delay: 60ms">
					<div class="px-4 py-3 border-b border-slate-100 flex items-center justify-between">
							<h2 class="text-[11px] font-semibold tracking-[0.1em] uppercase text-slate-400">
								Seguimiento
							</h2>
							{#if cargandoSeguimiento}
								<span class="w-3 h-3 border-2 border-slate-200 border-t-slate-400 rounded-full animate-spin-fast"></span>
							{/if}
						</div>

						{#if cargandoSeguimiento}
							<div class="divide-y divide-slate-50">
								{#each { length: 2 } as _, i (i)}
									<div class="px-4 py-3">
										<div class="h-3 w-2/3 rounded animate-shimmer mb-2"
											style="background: linear-gradient(90deg,#e2e8f0 25%,#f0f4f8 50%,#e2e8f0 75%); background-size:400% 100%"></div>
										<div class="h-2.5 w-1/2 rounded animate-shimmer"
											style="background: linear-gradient(90deg,#e2e8f0 25%,#f0f4f8 50%,#e2e8f0 75%); background-size:400% 100%"></div>
									</div>
								{/each}
							</div>

						{:else if errorSeguimiento}
							<p class="px-4 py-3 text-[12px] text-red-500">{errorSeguimiento}</p>

						{:else}
							<!-- Compras relacionadas -->
							{#if seguimientoCompras.length > 0}
								<div class="px-4 pt-3 pb-1">
									<p class="text-[10px] font-semibold uppercase tracking-wide text-slate-400 mb-2">
										Compras ({seguimientoCompras.length})
									</p>
								</div>
								<div class="divide-y divide-slate-50">
									{#each seguimientoCompras as c (c.numdoc)}
										{@const st = statusInfo(c.status)}
										<div class="px-4 py-2">
											<div class="flex items-center justify-between mb-0.5">
												<div class="flex items-center gap-2">
													<span class="px-1.5 py-0.5 rounded text-[10px] font-bold text-white bg-cyan-600">
														{c.tipodoc}
													</span>
													<span class="font-mono text-[12px] font-medium text-navy">{c.numdoc}</span>
												</div>
												<span class="text-[11px] text-slate-400">{formatFechaConDia(c.fecha)}</span>
											</div>
											<div class="flex items-center justify-between">
												<span class="text-[11px] text-slate-500">
													{c.arts_matched} de {c.total_arts} arts · {Math.round(c.coverage_pct)}%
												</span>
												<span class="px-2 py-0.5 rounded-full text-[10px] font-medium {st.cls}">
													{st.label}
												</span>
											</div>
										</div>
									{/each}
								</div>
							{/if}

							<!-- Remisiones relacionadas -->
							{#if seguimientoRemisiones.length > 0}
								<div class="px-4 pt-3 pb-1 {seguimientoCompras.length > 0 ? 'border-t border-slate-100' : ''}">
									<p class="text-[10px] font-semibold uppercase tracking-wide text-slate-400 mb-2">
										Remisiones ({seguimientoRemisiones.length})
									</p>
								</div>
								<div class="divide-y divide-slate-50">
									{#each seguimientoRemisiones as r (r.numdoc)}
										{@const st = statusInfo(r.status)}
										<div class="px-4 py-2">
											<div class="flex items-center justify-between mb-0.5">
												<div class="flex items-center gap-2">
													<span class="px-1.5 py-0.5 rounded text-[10px] font-bold text-white bg-blue-600">
														{r.tipodoc}
													</span>
													<span class="font-mono text-[12px] font-medium text-navy">{r.numdoc}</span>
												</div>
												<span class="text-[11px] text-slate-400">{formatFechaConDia(r.fecha)}</span>
											</div>
											<div class="flex items-center justify-between">
												<span class="text-[11px] text-slate-500">
													{r.arts_matched} de {r.total_arts} arts · {Math.round(r.coverage_pct)}%
												</span>
												<span class="px-2 py-0.5 rounded-full text-[10px] font-medium {st.cls}">
													{st.label}
												</span>
											</div>
										</div>
									{/each}
								</div>
							{/if}

							{#if seguimientoCompras.length === 0 && seguimientoRemisiones.length === 0}
								<p class="px-4 py-4 text-[12px] text-slate-400 text-center">
									Sin compras ni remisiones relacionadas
								</p>
							{/if}
						{/if}
					</div>

				<!-- Partidas -->
				<div class="bg-surface rounded-card shadow-card mb-3 overflow-hidden animate-fadeSlide" style="animation-delay: 80ms">
					<div class="px-4 py-2 border-b border-slate-100">
						<h2 class="text-[11px] font-semibold tracking-[0.1em] uppercase text-slate-400">
							Partidas ({movimientos.length})
						</h2>
					</div>
					<div class="divide-y divide-slate-50">
						{#each movimientos as mov (mov.numart + mov.numpar)}
							<div class="px-4 py-1.5">
								<div class="flex items-baseline justify-between">
									<span class="font-mono text-[12px] font-medium text-navy">{mov.numart.trim()}</span>
									<span class="ml-3 shrink-0 text-[12px] text-slate-700 font-medium">
										{mov.cant}
										<span class="text-[11px] font-normal text-slate-400">{articulos[mov.numart]?.unidad || mov.unidad.trim()}</span>
									</span>
								</div>
								{#if articulos[mov.numart]?.desc}
									<p class="text-[11px] text-slate-400 truncate">{articulos[mov.numart].desc}</p>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}
		{/if}
	</div>
</div>
