<script lang="ts">
	import { onMount } from 'svelte';
	import JsBarcode from 'jsbarcode';
	import { listArticulosEtiqueta } from '../lib/grpc.js';
	import type { ArticuloEtiqueta } from '../lib/types.js';

	// ── Estado panel izquierdo ─────────────────────────────────
	let query = $state('');
	let articulos = $state<ArticuloEtiqueta[]>([]);
	let cargando = $state(false);
	let errorMsg = $state('');
	let nextPageToken = $state('');
	let cargandoMas = $state(false);
	let mounted = false;

	// ── Estado panel derecho (cola de impresión) ───────────────
	interface ItemCola {
		articulo: ArticuloEtiqueta;
		cantidad: number;
	}
	let cola = $state<ItemCola[]>([]);
	let totalEtiquetas = $derived(cola.reduce((s, i) => s + i.cantidad, 0));

	// ── Búsqueda con debounce ──────────────────────────────────
	onMount(() => {
		mounted = true;
		buscar();
	});

	$effect(() => {
		void query;
		if (!mounted) return;
		const t = setTimeout(buscar, 300);
		return () => clearTimeout(t);
	});

	async function buscar() {
		cargando = true;
		errorMsg = '';
		nextPageToken = '';
		try {
			const res = await listArticulosEtiqueta(query.trim() || undefined);
			articulos = res.articulos;
			nextPageToken = res.next_page_token;
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		} finally {
			cargando = false;
		}
	}

	async function cargarMas() {
		if (!nextPageToken || cargandoMas) return;
		cargandoMas = true;
		try {
			const res = await listArticulosEtiqueta(query.trim() || undefined, nextPageToken);
			articulos = [...articulos, ...res.articulos];
			nextPageToken = res.next_page_token;
		} catch {
			// silencio
		} finally {
			cargandoMas = false;
		}
	}

	// ── Cola de impresión ──────────────────────────────────────
	function agregarACola(art: ArticuloEtiqueta) {
		const idx = cola.findIndex((i) => i.articulo.numart === art.numart);
		if (idx >= 0) {
			cola[idx].cantidad += 1;
		} else {
			cola = [...cola, { articulo: art, cantidad: 1 }];
		}
	}

	function quitarDeCola(numart: string) {
		cola = cola.filter((i) => i.articulo.numart !== numart);
	}

	function setCantidad(numart: string, val: number) {
		const idx = cola.findIndex((i) => i.articulo.numart === numart);
		if (idx >= 0) {
			if (val < 1) {
				quitarDeCola(numart);
			} else {
				cola[idx].cantidad = val;
			}
		}
	}

	// ── Impresión ──────────────────────────────────────────────
	function escapeHtml(s: string): string {
		return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	}

	function imprimir() {
		const barcodeOpts = { format: 'CODE128', width: 2, height: 50, displayValue: false, margin: 0 };

		const css = `* { box-sizing: border-box; margin: 0; padding: 0; }
			@page { size: 62mm auto; margin: 3mm; }
			.label-page { page-break-after: always; break-after: page; width: 56mm; padding: 2mm; font-family: monospace, sans-serif; }
			.label-barcode { width: 100%; margin-bottom: 2mm; }
			.label-barcode svg { width: 100%; height: auto; }
			.label-numart { font-size: 14pt; font-weight: bold; letter-spacing: 0.05em; margin-bottom: 1mm; text-align: center; }
			.label-desc { font-size: 9pt; line-height: 1.3; font-family: "Arial Narrow", "Helvetica Neue", sans-serif; font-stretch: condensed; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }`;

		const items = cola.flatMap((item) =>
			Array.from({ length: item.cantidad }, () => item.articulo)
		);

		const labels = items.map((art) => {
			let barcodeSvg = '';
			if (art.codigo) {
				const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
				try {
					JsBarcode(svg, art.codigo, barcodeOpts);
					barcodeSvg = svg.outerHTML;
				} catch {
					// código inválido para Code128
				}
			}
			return `<div class="label-page">
				${barcodeSvg ? `<div class="label-barcode">${barcodeSvg}</div>` : ''}
				<div class="label-numart">${escapeHtml(art.numart)}</div>
				<div class="label-desc">${escapeHtml(art.desc)}</div>
			</div>`;
		});

		const html = `<!DOCTYPE html><html><head><meta charset="UTF-8"><style>${css}</style></head><body>${labels.join('')}</body></html>`;

		const iframe = document.createElement('iframe');
		iframe.style.cssText = 'position:fixed;width:0;height:0;border:0;top:0;left:0;';
		document.body.appendChild(iframe);

		const doc = iframe.contentDocument!;
		doc.open();
		doc.write(html);
		doc.close();

		iframe.contentWindow!.print();
		setTimeout(() => document.body.removeChild(iframe), 2000);
	}
</script>

<!-- ── Wrapper ───────────────────────────────────────────── -->
<div class="flex flex-col h-screen overflow-hidden bg-bg">

	<!-- ── Header oscuro estándar ────────────────────────────── -->
	<div class="bg-[#0f1f38] px-4 pt-5 pb-4 md:px-6 flex-shrink-0">
		<div class="flex items-center justify-between">
			<div>
				<p class="text-[11px] font-semibold tracking-[0.12em] uppercase text-white/40">Inventario</p>
				<h1 class="font-barlow-condensed text-[22px] font-bold text-white leading-none">Etiquetas</h1>
			</div>

			{#if cola.length > 0}
				<div class="flex items-center gap-2">
					<button
						onclick={() => (cola = [])}
						class="border border-white/20 text-white/70 hover:bg-white/10 active:bg-white/20
							   px-3 py-1.5 rounded-lg text-[12px] font-medium font-barlow transition-colors"
					>
						Limpiar
					</button>
					<button
						onclick={imprimir}
						class="flex items-center gap-1.5 bg-navy hover:bg-navy-light active:bg-navy-dark
							   text-white px-3 py-1.5 rounded-lg text-[12px] font-semibold font-barlow transition-colors"
					>
						<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<polyline points="6 9 6 2 18 2 18 9"/>
							<path d="M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2"/>
							<rect x="6" y="14" width="12" height="8"/>
						</svg>
						Imprimir
						<span class="ml-0.5 bg-white/20 text-white text-[10px] font-mono font-bold px-1.5 py-0.5 rounded-full leading-none">
							{totalEtiquetas}
						</span>
					</button>
				</div>
			{/if}
		</div>

	</div>
	<div class="h-px flex-shrink-0" style="background: linear-gradient(90deg, rgba(255,255,255,0.08) 0%, transparent 100%)"></div>

	<!-- ── Cuerpo: 2 paneles ──────────────────────────────────── -->
	<div class="flex flex-1 overflow-hidden">

		<!-- ── Panel izquierdo: catálogo ──────────────────────── -->
		<div class="flex flex-col w-[55%] overflow-hidden bg-surface border-r border-slate-200">

			<!-- Header panel -->
			<div class="px-4 py-3 border-b border-slate-200 flex-shrink-0">
				<div class="flex items-center gap-2.5 mb-2.5">
					<div class="w-7 h-7 rounded-lg flex items-center justify-center bg-navy/[0.08] border border-navy/20 text-navy flex-shrink-0">
						<svg class="w-[14px] h-[14px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">
							<path d="M3 5h2M7 5h1M12 5h3M17 5h1M3 10h1M6 10h4M12 10h1M15 10h3M3 15h2M7 15h1M12 15h3M17 15h1M3 19h4M9 19h1M12 19h4M18 19h1"/>
						</svg>
					</div>
					<div>
						<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400">Catálogo</p>
						<p class="font-mono text-[11px] text-slate-500 mt-0.5">
							{#if cargando && articulos.length === 0}
								cargando…
							{:else}
								{articulos.length} artículo{articulos.length !== 1 ? 's' : ''}{nextPageToken ? '+' : ''}
							{/if}
						</p>
					</div>
				</div>

				<!-- Search -->
				<div class="relative">
					<svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-slate-400 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="11" cy="11" r="8"/>
						<line x1="21" y1="21" x2="16.65" y2="16.65"/>
					</svg>
					<input
						type="text"
						placeholder="NUMART, descripción o código de barras…"
						value={query}
						oninput={(e) => (query = e.currentTarget.value.toUpperCase())}
						class="w-full pl-8 pr-8 py-1.5 rounded-lg bg-slate-100 border border-slate-200
							   text-[12px] text-slate-700 placeholder:text-slate-400 uppercase
							   focus:outline-none focus:ring-2 focus:ring-navy/20 focus:border-navy/40
							   focus:bg-white transition-all"
					/>
					{#if query}
						<button
							class="absolute right-2.5 top-1/2 -translate-y-1/2 w-4 h-4 flex items-center justify-center
								   text-slate-400 hover:text-slate-600 transition-colors"
							onclick={() => (query = '')}
							aria-label="Limpiar búsqueda"
						>
							<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" class="w-3 h-3">
								<line x1="18" y1="6" x2="6" y2="18"/>
								<line x1="6" y1="6" x2="18" y2="18"/>
							</svg>
						</button>
					{/if}
				</div>
			</div>

			<!-- Lista -->
			<div class="flex-1 overflow-y-auto scrollbar-thin">
				{#if cargando && articulos.length === 0}
					<div class="flex flex-col items-center justify-center h-full min-h-[180px] gap-2 p-6">
						<div class="flex gap-1.5 mb-1">
							<span class="w-1.5 h-1.5 rounded-full bg-slate-300 animate-pulse [animation-delay:0ms]"></span>
							<span class="w-1.5 h-1.5 rounded-full bg-slate-300 animate-pulse [animation-delay:200ms]"></span>
							<span class="w-1.5 h-1.5 rounded-full bg-slate-300 animate-pulse [animation-delay:400ms]"></span>
						</div>
						<p class="text-[12px] text-slate-400">Cargando artículos…</p>
					</div>

				{:else if errorMsg}
					<div class="flex flex-col items-center justify-center h-full min-h-[180px] gap-2 p-6">
						<svg class="w-9 h-9 text-red-400 mb-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
							<circle cx="12" cy="12" r="10"/>
							<line x1="12" y1="8" x2="12" y2="12"/>
							<line x1="12" y1="16" x2="12.01" y2="16"/>
						</svg>
						<p class="text-[12px] text-red-500 text-center max-w-[280px] leading-relaxed">{errorMsg}</p>
						<button
							class="mt-1 px-3.5 py-1 text-[11.5px] text-red-600 bg-red-50 border border-red-200 rounded-lg
								   hover:bg-red-100 transition-colors"
							onclick={buscar}
						>Reintentar</button>
					</div>

				{:else if articulos.length === 0}
					<div class="flex flex-col items-center justify-center h-full min-h-[180px] gap-1.5 p-6">
						<svg class="w-10 h-10 text-slate-300 mb-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.9" stroke-linecap="round" stroke-linejoin="round">
							<circle cx="11" cy="11" r="8"/>
							<line x1="21" y1="21" x2="16.65" y2="16.65"/>
						</svg>
						<p class="text-[12px] text-slate-400 text-center">Sin resultados para "{query}"</p>
					</div>

				{:else}
					<div>
						{#each articulos as art (art.numart)}
							<button
								onclick={() => agregarACola(art)}
								class="flex items-start gap-2.5 w-full text-left px-4 py-2.5
									   border-b border-slate-100 border-l-2 border-l-transparent
									   hover:bg-slate-50 hover:border-l-navy transition-all group"
							>
								<div class="flex-1 min-w-0">
									<div class="mb-1">
										<span class="inline-block font-mono text-[10px] font-bold tracking-wide
											         text-amber-800 bg-amber-100 border border-amber-300
											         px-1.5 py-0.5 rounded leading-none">
											{art.numart}
										</span>
									</div>
									<p class="text-[12px] text-slate-600 leading-snug overflow-hidden text-ellipsis whitespace-nowrap mb-1">
										{art.desc}
									</p>
									{#if art.codigo}
										<span class="inline-flex items-center gap-1 font-mono text-[9.5px] font-semibold
											         text-teal-700 bg-teal-50 border border-teal-200
											         px-1.5 py-0.5 rounded leading-none">
											<svg class="w-[9px] h-[9px] shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
												<path d="M3 5h2M7 5h1M12 5h3M17 5h1M3 10h1M6 10h4M12 10h1M15 10h3M3 15h2M7 15h1M12 15h3M17 15h1M3 19h4M9 19h1M12 19h4M18 19h1"/>
											</svg>
											{art.codigo}
										</span>
									{:else}
										<span class="inline-flex items-center gap-1 font-mono text-[9.5px] font-semibold
											         text-red-500 bg-red-50 border border-red-200
											         px-1.5 py-0.5 rounded leading-none">
											<svg class="w-[9px] h-[9px] shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
												<circle cx="12" cy="12" r="10"/>
												<line x1="12" y1="8" x2="12" y2="12"/>
												<line x1="12" y1="16" x2="12.01" y2="16"/>
											</svg>
											Sin código de barras
										</span>
									{/if}
								</div>
								<div class="flex-shrink-0 w-5 h-5 rounded-full bg-slate-100 flex items-center justify-center
									        text-slate-400 mt-0.5 group-hover:bg-navy/[0.08] group-hover:text-navy transition-all">
									<svg class="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
										<line x1="12" y1="5" x2="12" y2="19"/>
										<line x1="5" y1="12" x2="19" y2="12"/>
									</svg>
								</div>
							</button>
						{/each}

						{#if nextPageToken}
							<div class="p-4 flex justify-center">
								<button
									onclick={cargarMas}
									disabled={cargandoMas}
									class="px-4 py-1.5 text-[11.5px] text-slate-500 border border-slate-200 rounded-lg
										   hover:border-slate-300 hover:text-slate-700 hover:bg-slate-50
										   transition-all disabled:opacity-40 disabled:cursor-not-allowed"
								>
									{cargandoMas ? 'Cargando…' : 'Cargar más artículos'}
								</button>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>

		<!-- ── Panel derecho: cola de impresión ───────────────── -->
		<div class="flex flex-col flex-1 overflow-hidden bg-bg">

			<!-- Header panel -->
			<div class="px-4 py-3 border-b border-slate-200 flex-shrink-0 bg-surface">
				<div class="flex items-center gap-3">
					<div class="flex items-center gap-2.5">
						<div class="w-7 h-7 rounded-lg flex items-center justify-center bg-navy/[0.08] border border-navy/20 text-navy flex-shrink-0">
							<svg class="w-[14px] h-[14px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
								<polyline points="6 9 6 2 18 2 18 9"/>
								<path d="M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2"/>
								<rect x="6" y="14" width="12" height="8"/>
							</svg>
						</div>
						<div>
							<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400">Impresión</p>
							<p class="font-mono text-[11px] text-slate-500 mt-0.5 flex items-center gap-1.5">
								{#if totalEtiquetas > 0}
									<span class="inline-flex items-center justify-center min-w-[18px] h-[16px] px-1.5
										         bg-navy text-white rounded-full text-[10px] font-bold leading-none">
										{totalEtiquetas}
									</span>
									etiqueta{totalEtiquetas !== 1 ? 's' : ''} en cola
								{:else}
									cola vacía
								{/if}
							</p>
						</div>
					</div>

					</div>
			</div>

			<!-- Cola -->
			<div class="flex-1 overflow-y-auto scrollbar-thin">
				{#if cola.length === 0}
					<div class="flex flex-col items-center justify-center h-full min-h-[180px] gap-1.5 p-6">
						<svg class="w-16 h-16 text-slate-300 mb-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.6" stroke-linecap="round" stroke-linejoin="round">
							<polyline points="6 9 6 2 18 2 18 9"/>
							<path d="M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2"/>
							<rect x="6" y="14" width="12" height="8"/>
						</svg>
						<p class="font-barlow-condensed text-[14px] font-bold text-slate-400 tracking-wide uppercase">
							Cola vacía
						</p>
						<p class="text-[11.5px] text-slate-400 text-center leading-relaxed">
							Selecciona artículos del catálogo<br/>para agregarlos a la lista de impresión
						</p>
					</div>
				{:else}
					<div class="p-3 flex flex-col gap-1.5">
						{#each cola as item (item.articulo.numart)}
							<div class="flex items-center gap-2.5 px-3 py-2.5 bg-surface border border-slate-200
								        rounded-card shadow-card hover:shadow-card-md transition-all">
								<div class="flex-1 min-w-0">
									<span class="inline-block font-mono text-[10px] font-bold tracking-wide
										         text-amber-800 bg-amber-100 border border-amber-300
										         px-1.5 py-0.5 rounded leading-none mb-1">
										{item.articulo.numart}
									</span>
									<p class="text-[12px] text-slate-600 leading-snug overflow-hidden text-ellipsis whitespace-nowrap">
										{item.articulo.desc}
									</p>
									{#if item.articulo.codigo}
										<p class="font-mono text-[9.5px] text-slate-400 mt-0.5">{item.articulo.codigo}</p>
									{/if}
								</div>

								<!-- Stepper de cantidad -->
								<div class="flex items-center border border-slate-200 rounded-lg overflow-hidden flex-shrink-0">
									<button
										onclick={() => setCantidad(item.articulo.numart, item.cantidad - 1)}
										class="w-6 h-7 flex items-center justify-center bg-slate-50 text-slate-500 text-[15px] leading-none
											   hover:bg-slate-100 hover:text-slate-700 transition-colors"
									>−</button>
									<input
										type="number"
										min="1"
										value={item.cantidad}
										oninput={(e) => {
											const v = parseInt((e.target as HTMLInputElement).value);
											if (!isNaN(v)) setCantidad(item.articulo.numart, v);
										}}
										class="qty-input w-8 h-7 text-center text-[12px] font-mono font-semibold text-slate-700
											   bg-white border-x border-slate-200 outline-none"
									/>
									<button
										onclick={() => setCantidad(item.articulo.numart, item.cantidad + 1)}
										class="w-6 h-7 flex items-center justify-center bg-slate-50 text-slate-500 text-[15px] leading-none
											   hover:bg-slate-100 hover:text-slate-700 transition-colors"
									>+</button>
								</div>

								<button
									onclick={() => quitarDeCola(item.articulo.numart)}
									class="flex-shrink-0 w-6 h-6 flex items-center justify-center rounded-md
										   text-slate-400 hover:bg-red-50 hover:text-red-500 transition-all"
									aria-label="Quitar de la cola"
								>
									<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
										<line x1="18" y1="6" x2="6" y2="18"/>
										<line x1="6" y1="6" x2="18" y2="18"/>
									</svg>
								</button>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>

<style>
	.qty-input::-webkit-inner-spin-button,
	.qty-input::-webkit-outer-spin-button {
		-webkit-appearance: none;
	}
</style>
