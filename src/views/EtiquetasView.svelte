<script lang="ts">
	import { onMount } from 'svelte';
	import JsBarcode from 'jsbarcode';
	import { invoke } from '@tauri-apps/api/core';
	import { listArticulosEtiqueta } from '../lib/grpc.js';
	import { getDefaultPrinter } from '../lib/dbf.js';
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

	// ── Impresora ──────────────────────────────────────────────
	let printers = $state<string[]>([]);
	let printerName = $state('');
	let printerModal = $state<{ title: string; body: string } | null>(null);

	// ── Búsqueda con debounce ──────────────────────────────────
	onMount(() => {
		mounted = true;
		buscar();
		Promise.all([
			invoke<string[]>('list_printers'),
			getDefaultPrinter(),
		]).then(([list, saved]) => {
			printers = list;
			printerName = saved ?? list[0] ?? '';
		}).catch(() => {});
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

	async function renderLabelToCanvas(
		art: ArticuloEtiqueta,
		barcodeSvg: string | null,
		heightMm: number
	): Promise<string> {
		const DPI = 96;
		const SCALE = 4; // ~384 DPI efectivo — buena calidad al escalar a 300 DPI del driver
		const W = Math.round((62 * DPI) / 25.4) * SCALE;
		const H = Math.round((heightMm * DPI) / 25.4) * SCALE;
		const pad = Math.round((3 * DPI) / 25.4) * SCALE;
		const innerW = W - 2 * pad;

		const canvas = document.createElement('canvas');
		canvas.width = W;
		canvas.height = H;
		const ctx = canvas.getContext('2d')!;
		ctx.textBaseline = 'top';

		ctx.fillStyle = 'white';
		ctx.fillRect(0, 0, W, H);

		let y = pad;

		if (barcodeSvg) {
			const img = new Image();
			const blob = new Blob([barcodeSvg], { type: 'image/svg+xml' });
			const url = URL.createObjectURL(blob);
			await new Promise<void>((res) => {
				img.onload = () => res();
				img.onerror = () => res();
				img.src = url;
			});
			const barcodeH = Math.round((14 * DPI) / 25.4) * SCALE;
			ctx.drawImage(img, pad, y, innerW, barcodeH);
			URL.revokeObjectURL(url);
			y += barcodeH + Math.round((2 * DPI) / 25.4) * SCALE;
		}

		const numartPx = Math.round((14 * DPI) / 72) * SCALE;
		ctx.font = `bold ${numartPx}px Arial, sans-serif`;
		ctx.fillStyle = 'black';
		ctx.textAlign = 'center';
		ctx.fillText(art.numart, W / 2, y);
		y += numartPx + Math.round((1 * DPI) / 25.4) * SCALE;

		const descPx = Math.round((6 * DPI) / 72) * SCALE;
		ctx.font = `${descPx}px "Arial Narrow", Arial, sans-serif`;
		ctx.textAlign = 'left';
		let desc = art.desc;
		if (ctx.measureText(desc).width > innerW) {
			while (desc.length > 0 && ctx.measureText(desc + '…').width > innerW) {
				desc = desc.slice(0, -1);
			}
			desc += '…';
		}
		ctx.fillText(desc, pad, y);

		return canvas.toDataURL('image/png').split(',')[1];
	}

	async function imprimir() {
		if (!printerName || !printers.includes(printerName)) {
			printerModal = {
				title: 'Impresora no disponible',
				body: printerName
					? `La impresora "${printerName}" no está instalada en este equipo.`
					: 'No hay impresora configurada.',
			};
			return;
		}
		const barcodeOpts = { format: 'CODE128', width: 2, height: 50, displayValue: false, margin: 0 };

		const items = cola.flatMap((item) =>
			Array.from({ length: item.cantidad }, () => item.articulo)
		);

		const rendered = items.map((art) => {
			let barcodeSvg: string | null = null;
			if (art.codigo) {
				const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
				try {
					JsBarcode(svg, art.codigo, barcodeOpts);
					barcodeSvg = svg.outerHTML;
				} catch {
					// código inválido para Code128
				}
			}
			return { art, barcodeSvg };
		});

		// Medir altura real del layout en un iframe oculto
		const baseCss = `* { box-sizing: border-box; margin: 0; padding: 0; }
			.label-page { width: 62mm; padding: 3mm; font-family: monospace, sans-serif; }
			.label-barcode { width: 100%; margin-bottom: 2mm; }
			.label-barcode svg { width: 100%; height: auto; }
			.label-numart { font-size: 14pt; font-weight: bold; font-family: Arial, sans-serif; letter-spacing: 0.05em; margin-bottom: 1mm; text-align: center; }
			.label-desc { font-size: 6pt; line-height: 1.3; font-family: "Arial Narrow", "Helvetica Neue", sans-serif; font-stretch: condensed; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }`;

		const firstItem = rendered[0];
		const sampleDiv = `<div class="label-page">
			${firstItem.barcodeSvg ? `<div class="label-barcode">${firstItem.barcodeSvg}</div>` : ''}
			<div class="label-numart">${escapeHtml(firstItem.art.numart)}</div>
			<div class="label-desc">${escapeHtml(firstItem.art.desc)}</div>
		</div>`;
		const measureHtml = `<!DOCTYPE html><html><head><meta charset="UTF-8"><style>${baseCss}</style></head><body>${sampleDiv}</body></html>`;

		const iframe = document.createElement('iframe');
		iframe.style.cssText = 'position:fixed;width:62mm;top:-9999px;left:0;border:0;visibility:hidden;';
		document.body.appendChild(iframe);

		const doc = iframe.contentDocument!;
		doc.open();
		doc.write(measureHtml);
		doc.close();

		await new Promise<void>((r) => setTimeout(r, 80));

		const labelEl = doc.querySelector('.label-page') as HTMLElement | null;
		const heightPx = labelEl ? labelEl.offsetHeight : doc.body.scrollHeight;
		const heightMm = Math.ceil(heightPx * 25.4 / 96);

		document.body.removeChild(iframe);

		// Renderizar cada etiqueta en canvas (calidad ~384 DPI) y enviar PNGs a Rust/GDI
		const labels = await Promise.all(
			rendered.map(({ art, barcodeSvg }) =>
				renderLabelToCanvas(art, barcodeSvg, heightMm).then((b) => ({ png_b64: b }))
			)
		);

		try {
			await invoke('print_etiquetas', { labels, heightMm, printerName });
			cola = [];
		} catch (e) {
			const msg = e instanceof Error ? e.message : String(e);
			printerModal = { title: 'Error de impresión', body: msg };
		}
	}
</script>

<!-- ── Wrapper ───────────────────────────────────────────── -->
<div class="flex flex-col h-screen overflow-hidden bg-bg">


	<!-- ── Cuerpo: 2 paneles ──────────────────────────────────── -->
	<div class="flex flex-1 overflow-hidden bg-[#0f1f38]">

		<!-- ── Panel izquierdo: catálogo ──────────────────────── -->
		<div class="flex flex-col w-[55%] overflow-hidden bg-[#d4dbe8] border-r border-white/10">

			<!-- Header panel -->
			<div class="px-4 pt-3 pb-3 border-b border-white/10 flex-shrink-0 bg-[#0f1f38]">
				<div class="flex items-center gap-2.5 mb-2.5">
					<div class="w-7 h-7 rounded-lg flex items-center justify-center bg-white/10 border border-white/20 text-white flex-shrink-0">
						<svg class="w-[14px] h-[14px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">
							<path d="M3 5h2M7 5h1M12 5h3M17 5h1M3 10h1M6 10h4M12 10h1M15 10h3M3 15h2M7 15h1M12 15h3M17 15h1M3 19h4M9 19h1M12 19h4M18 19h1"/>
						</svg>
					</div>
					<div>
						<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-white/40">Catálogo</p>
						<p class="font-mono text-[11px] text-white/70 mt-0.5">
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
					<svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-navy pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="11" cy="11" r="8"/>
						<line x1="21" y1="21" x2="16.65" y2="16.65"/>
					</svg>
					<input
						type="text"
						placeholder="NUMART, descripción o código de barras…"
						value={query}
						oninput={(e) => (query = e.currentTarget.value.toUpperCase())}
						class="w-full pl-8 pr-8 py-1.5 rounded-lg bg-white text-slate-800 border border-transparent
							   text-[12px] placeholder:text-slate-400 uppercase
							   focus:outline-none focus:ring-2 focus:ring-white/30 transition-all"
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
					<div class="p-2 flex flex-col gap-1.5">
						{#each articulos as art (art.numart)}
							<button
								onclick={() => agregarACola(art)}
								class="flex items-center gap-2.5 w-full text-left px-3 py-2.5
									   bg-white border border-slate-200 rounded-lg shadow-sm
									   hover:border-navy/40 hover:bg-navy/[0.03] transition-all group"
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
								<div class="flex-shrink-0 w-6 h-6 rounded-full bg-navy flex items-center justify-center
									        text-white transition-all group-hover:scale-110">
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
									class="px-4 py-1.5 text-[11.5px] font-medium bg-navy text-white rounded-lg
										   hover:bg-[#2a4a8a] active:bg-[#0a1525] transition-all disabled:opacity-40 disabled:cursor-not-allowed"
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
			<div class="px-4 py-3 border-b border-white/10 flex-shrink-0 bg-[#0f1f38]">
				<div class="flex items-center justify-between gap-3">
					<div class="flex items-center gap-2.5">
						<div class="w-7 h-7 rounded-lg flex items-center justify-center bg-white/10 border border-white/20 text-white flex-shrink-0">
							<svg class="w-[14px] h-[14px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
								<polyline points="6 9 6 2 18 2 18 9"/>
								<path d="M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2"/>
								<rect x="6" y="14" width="12" height="8"/>
							</svg>
						</div>
						<div>
							<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-white/40">Impresión</p>
							<p class="font-mono text-[11px] text-white/70 mt-0.5 flex items-center gap-1.5">
								{#if totalEtiquetas > 0}
									<span class="inline-flex items-center justify-center min-w-[18px] h-[16px] px-1.5
										         bg-white/20 text-white rounded-full text-[10px] font-bold leading-none">
										{totalEtiquetas}
									</span>
									etiqueta{totalEtiquetas !== 1 ? 's' : ''} en lista
								{:else}
									lista vacía
								{/if}
							</p>
						</div>
					</div>

					<!-- Acciones de impresión -->
					<div class="flex items-center gap-2 flex-shrink-0">
						{#if cola.length > 0}
							<button
								onclick={() => (cola = [])}
								class="border border-white/20 text-white/70 hover:bg-white/10 active:bg-white/20
									   px-3 py-1.5 rounded-lg text-[12px] font-medium font-barlow transition-colors"
							>
								Limpiar
							</button>
							<button
								onclick={imprimir}
								class="flex items-center gap-1.5 bg-white hover:bg-white/90 active:bg-white/80
									   text-navy px-3 py-1.5 rounded-lg text-[12px] font-semibold font-barlow transition-colors"
							>
								<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
									<polyline points="6 9 6 2 18 2 18 9"/>
									<path d="M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2"/>
									<rect x="6" y="14" width="12" height="8"/>
								</svg>
								Imprimir
								<span class="ml-0.5 bg-navy/10 text-navy text-[10px] font-mono font-bold px-1.5 py-0.5 rounded-full leading-none">
									{totalEtiquetas}
								</span>
							</button>
						{/if}
					</div>
				</div>
				<!-- replica el buscador del panel izquierdo para igualar la altura del header -->
				<div class="mt-2.5" aria-hidden="true">
					<div class="w-full pl-8 pr-8 py-1.5 text-[12px] border border-transparent invisible">phantom</div>
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
							Lista vacía
						</p>
						<p class="text-[11.5px] text-slate-400 text-center leading-relaxed">
							Selecciona artículos del catálogo<br/>para agregarlos a la Lista de Impresión
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
									{:else}
										<span class="inline-flex items-center gap-1 font-mono text-[9.5px] font-semibold
											         text-red-500 bg-red-50 border border-red-200
											         px-1.5 py-0.5 rounded leading-none mt-0.5">
											<svg class="w-[9px] h-[9px] shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
												<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
												<line x1="12" y1="9" x2="12" y2="13"/>
												<line x1="12" y1="17" x2="12.01" y2="17"/>
											</svg>
											Sin código de barras
										</span>
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

<!-- Modal: error de impresora -->
{#if printerModal}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
		<div class="bg-white rounded-2xl shadow-xl w-[360px] p-6 flex flex-col gap-4">
			<div class="flex items-start gap-3">
				<div class="w-9 h-9 rounded-full bg-red-100 flex items-center justify-center shrink-0 mt-0.5">
					<svg class="w-5 h-5 text-red-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="12" cy="12" r="10"/>
						<line x1="12" y1="8" x2="12" y2="12"/>
						<line x1="12" y1="16" x2="12.01" y2="16"/>
					</svg>
				</div>
				<div class="min-w-0">
					<h3 class="font-barlow-condensed text-[16px] font-bold text-slate-800 leading-none mb-1">{printerModal.title}</h3>
					<p class="text-[12px] text-slate-500 leading-relaxed break-words">{printerModal.body}</p>
					{#if printerModal.title === 'Impresora no disponible'}
						<p class="text-[11px] text-slate-400 mt-1">Configúrala en <strong class="text-slate-500">Configuración → Impresión</strong>.</p>
					{/if}
				</div>
			</div>
			<button
				onclick={() => (printerModal = null)}
				class="w-full bg-navy text-white rounded-xl py-2 text-[13px] font-semibold font-barlow
					   hover:opacity-90 transition-opacity"
			>Entendido</button>
		</div>
	</div>
{/if}

<style>
	.qty-input::-webkit-inner-spin-button,
	.qty-input::-webkit-outer-spin-button {
		-webkit-appearance: none;
	}
</style>
