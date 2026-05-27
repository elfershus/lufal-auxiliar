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
		const items = cola.flatMap((item) =>
			Array.from({ length: item.cantidad }, () => item.articulo)
		);

		const labels = items.map((art) => {
			let barcodeSvg = '';
			if (art.codigo) {
				const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
				try {
					JsBarcode(svg, art.codigo, {
						format: 'CODE128',
						width: 2,
						height: 50,
						displayValue: false,
						margin: 0
					});
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

		const html = `<!DOCTYPE html><html><head><meta charset="UTF-8"><style>
			* { box-sizing: border-box; margin: 0; padding: 0; }
			@page { size: 90mm auto; margin: 3mm; }
			.label-page { page-break-after: always; break-after: page; width: 84mm; padding: 2mm; font-family: monospace, sans-serif; }
			.label-barcode { width: 100%; margin-bottom: 2mm; }
			.label-barcode svg { width: 100%; height: auto; }
			.label-numart { font-size: 14pt; font-weight: bold; letter-spacing: 0.05em; margin-bottom: 1mm; text-align: center; }
			.label-desc { font-size: 9pt; line-height: 1.3; font-family: "Arial Narrow", "Helvetica Neue", sans-serif; font-stretch: condensed; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
		</style></head><body>${labels.join('')}</body></html>`;

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

<!-- ── Layout principal ──────────────────────────────────── -->
<div class="view-root">

	<!-- ── Panel izquierdo: catálogo ──────────────────────── -->
	<div class="panel catalogue-panel">

		<!-- Header -->
		<div class="panel-header">
			<div class="flex items-center gap-2.5 mb-3">
				<div class="section-icon-wrap">
					<svg class="w-[15px] h-[15px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">
						<path d="M3 5h2M7 5h1M12 5h3M17 5h1M3 10h1M6 10h4M12 10h1M15 10h3M3 15h2M7 15h1M12 15h3M17 15h1M3 19h4M9 19h1M12 19h4M18 19h1"/>
					</svg>
				</div>
				<div>
					<h2 class="section-title">Catálogo</h2>
					<p class="section-sub">
						{#if cargando && articulos.length === 0}
							cargando…
						{:else}
							{articulos.length} artículo{articulos.length !== 1 ? 's' : ''}{nextPageToken ? '+' : ''}
						{/if}
					</p>
				</div>
			</div>

			<div class="search-wrap">
				<svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<circle cx="11" cy="11" r="8"/>
					<line x1="21" y1="21" x2="16.65" y2="16.65"/>
				</svg>
				<input
					type="text"
					placeholder="NUMART, descripción o código de barras…"
					bind:value={query}
					class="search-input"
				/>
				{#if query}
					<button class="search-clear" onclick={() => (query = '')} aria-label="Limpiar búsqueda">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
							<line x1="18" y1="6" x2="6" y2="18"/>
							<line x1="6" y1="6" x2="18" y2="18"/>
						</svg>
					</button>
				{/if}
			</div>
		</div>

		<!-- Lista -->
		<div class="panel-body">
			{#if cargando && articulos.length === 0}
				<div class="state-center">
					<div class="loading-dots">
						<span></span><span></span><span></span>
					</div>
					<p class="state-text">Cargando artículos…</p>
				</div>

			{:else if errorMsg}
				<div class="state-center">
					<div style="color:#f87171;opacity:.65;">
						<svg class="w-9 h-9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
							<circle cx="12" cy="12" r="10"/>
							<line x1="12" y1="8" x2="12" y2="12"/>
							<line x1="12" y1="16" x2="12.01" y2="16"/>
						</svg>
					</div>
					<p class="error-text">{errorMsg}</p>
					<button class="btn-retry" onclick={buscar}>Reintentar</button>
				</div>

			{:else if articulos.length === 0}
				<div class="state-center">
					<svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.9" stroke-linecap="round" stroke-linejoin="round">
						<circle cx="11" cy="11" r="8"/>
						<line x1="21" y1="21" x2="16.65" y2="16.65"/>
					</svg>
					<p class="state-text">Sin resultados para "{query}"</p>
				</div>

			{:else}
				<div>
					{#each articulos as art (art.numart)}
						<button onclick={() => agregarACola(art)} class="art-row">
							<div class="art-info">
								<div class="mb-1">
									<span class="numart-badge">{art.numart}</span>
								</div>
								<p class="art-desc">{art.desc}</p>
								{#if art.codigo}
									<span class="barcode-chip has-code">
										<svg class="w-[9px] h-[9px] shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
											<path d="M3 5h2M7 5h1M12 5h3M17 5h1M3 10h1M6 10h4M12 10h1M15 10h3M3 15h2M7 15h1M12 15h3M17 15h1M3 19h4M9 19h1M12 19h4M18 19h1"/>
										</svg>
										{art.codigo}
									</span>
								{:else}
									<span class="barcode-chip no-code">
										<svg class="w-[9px] h-[9px] shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
											<circle cx="12" cy="12" r="10"/>
											<line x1="12" y1="8" x2="12" y2="12"/>
											<line x1="12" y1="16" x2="12.01" y2="16"/>
										</svg>
										Sin código de barras
									</span>
								{/if}
							</div>
							<div class="add-btn">
								<svg class="w-[11px] h-[11px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
									<line x1="12" y1="5" x2="12" y2="19"/>
									<line x1="5" y1="12" x2="19" y2="12"/>
								</svg>
							</div>
						</button>
					{/each}

					{#if nextPageToken}
						<div class="p-4 flex justify-center">
							<button onclick={cargarMas} disabled={cargandoMas} class="btn-load-more">
								{cargandoMas ? 'Cargando…' : 'Cargar más artículos'}
							</button>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>

	<!-- ── Panel derecho: cola de impresión ───────────────── -->
	<div class="panel queue-panel">

		<!-- Header -->
		<div class="panel-header">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2.5">
					<div class="section-icon-wrap queue-icon">
						<svg class="w-[15px] h-[15px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
							<polyline points="6 9 6 2 18 2 18 9"/>
							<path d="M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2"/>
							<rect x="6" y="14" width="12" height="8"/>
						</svg>
					</div>
					<div>
						<h2 class="section-title">Impresión</h2>
						<p class="section-sub">
							{#if totalEtiquetas > 0}
								<span class="total-pill">{totalEtiquetas}</span>
								etiqueta{totalEtiquetas !== 1 ? 's' : ''} en cola
							{:else}
								cola vacía
							{/if}
						</p>
					</div>
				</div>

				{#if cola.length > 0}
					<div class="flex items-center gap-2">
						<button onclick={() => (cola = [])} class="btn-clear">Limpiar</button>
						<button onclick={imprimir} class="btn-print">
							<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<polyline points="6 9 6 2 18 2 18 9"/>
								<path d="M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2"/>
								<rect x="6" y="14" width="12" height="8"/>
							</svg>
							Imprimir
						</button>
					</div>
				{/if}
			</div>
		</div>

		<!-- Cola -->
		<div class="panel-body">
			{#if cola.length === 0}
				<div class="state-center queue-empty">
					<svg class="empty-printer-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="0.6" stroke-linecap="round" stroke-linejoin="round">
						<polyline points="6 9 6 2 18 2 18 9"/>
						<path d="M6 18H4a2 2 0 01-2-2v-5a2 2 0 012-2h16a2 2 0 012 2v5a2 2 0 01-2 2h-2"/>
						<rect x="6" y="14" width="12" height="8"/>
					</svg>
					<p class="empty-title">Cola vacía</p>
					<p class="empty-sub">
						Selecciona artículos del catálogo<br />para agregarlos a la lista de impresión
					</p>
				</div>
			{:else}
				<div class="queue-list">
					{#each cola as item (item.articulo.numart)}
						<div class="queue-card">
							<div class="queue-card-info">
								<span class="numart-badge">{item.articulo.numart}</span>
								<p class="queue-desc">{item.articulo.desc}</p>
								{#if item.articulo.codigo}
									<p class="queue-code">{item.articulo.codigo}</p>
								{/if}
							</div>

							<div class="qty-stepper">
								<button
									onclick={() => setCantidad(item.articulo.numart, item.cantidad - 1)}
									class="qty-btn"
								>−</button>
								<input
									type="number"
									min="1"
									value={item.cantidad}
									oninput={(e) => {
										const v = parseInt((e.target as HTMLInputElement).value);
										if (!isNaN(v)) setCantidad(item.articulo.numart, v);
									}}
									class="qty-input"
								/>
								<button
									onclick={() => setCantidad(item.articulo.numart, item.cantidad + 1)}
									class="qty-btn"
								>+</button>
							</div>

							<button
								onclick={() => quitarDeCola(item.articulo.numart)}
								class="remove-btn"
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

<style>
	/* ── Root & panels ───────────────────────────────────── */
	.view-root {
		display: flex;
		height: 100vh;
		overflow: hidden;
		background: #090e17;
	}

	.panel {
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.catalogue-panel {
		width: 55%;
		background: #0e1623;
		border-right: 1px solid rgba(255, 255, 255, 0.13);
	}

	.queue-panel {
		width: 45%;
		background: #0a1120;
	}

	/* ── Panel header ────────────────────────────────────── */
	.panel-header {
		padding: 14px 16px 12px;
		background: rgba(0, 0, 0, 0.2);
		border-bottom: 1px solid rgba(255, 255, 255, 0.07);
		flex-shrink: 0;
	}

	.section-icon-wrap {
		width: 30px;
		height: 30px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(20, 184, 166, 0.1);
		border: 1px solid rgba(20, 184, 166, 0.2);
		border-radius: 7px;
		color: #2dd4bf;
		flex-shrink: 0;
	}

	.section-icon-wrap.queue-icon {
		background: rgba(225, 29, 72, 0.1);
		border-color: rgba(225, 29, 72, 0.2);
		color: #fb7185;
	}

	.section-title {
		font-family: 'Barlow Condensed', sans-serif;
		font-size: 14px;
		font-weight: 700;
		color: rgba(255, 255, 255, 0.88);
		letter-spacing: 0.06em;
		text-transform: uppercase;
		line-height: 1;
		margin: 0;
	}

	.section-sub {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 10.5px;
		color: rgba(255, 255, 255, 0.45);
		margin-top: 4px;
		font-family: ui-monospace, monospace;
		line-height: 1;
	}

	.total-pill {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 18px;
		height: 16px;
		padding: 0 5px;
		background: #e11d48;
		color: white;
		border-radius: 8px;
		font-size: 10px;
		font-weight: 700;
		font-family: ui-monospace, monospace;
		line-height: 1;
	}

	/* ── Search ──────────────────────────────────────────── */
	.search-wrap {
		position: relative;
		display: flex;
		align-items: center;
	}

	.search-icon {
		position: absolute;
		left: 10px;
		width: 13px;
		height: 13px;
		color: rgba(255, 255, 255, 0.22);
		pointer-events: none;
	}

	.search-input {
		width: 100%;
		padding: 7px 32px 7px 30px;
		background: rgba(255, 255, 255, 0.05);
		border: 1px solid rgba(255, 255, 255, 0.09);
		border-radius: 7px;
		font-size: 12px;
		color: rgba(255, 255, 255, 0.8);
		outline: none;
		transition: border-color 0.15s, background 0.15s;
		font-family: inherit;
	}

	.search-input::placeholder {
		color: rgba(255, 255, 255, 0.32);
	}

	.search-input:focus {
		border-color: rgba(45, 212, 191, 0.4);
		background: rgba(255, 255, 255, 0.07);
	}

	.search-clear {
		position: absolute;
		right: 7px;
		width: 18px;
		height: 18px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: rgba(255, 255, 255, 0.25);
		background: none;
		border: none;
		cursor: pointer;
		padding: 0;
		transition: color 0.12s;
	}

	.search-clear svg {
		width: 10px;
		height: 10px;
	}

	.search-clear:hover {
		color: rgba(255, 255, 255, 0.65);
	}

	/* ── Panel body ──────────────────────────────────────── */
	.panel-body {
		flex: 1;
		overflow-y: auto;
		scrollbar-width: thin;
		scrollbar-color: rgba(255, 255, 255, 0.08) transparent;
	}

	/* ── States ──────────────────────────────────────────── */
	.state-center {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		min-height: 180px;
		padding: 24px;
		gap: 8px;
	}

	.state-text {
		font-size: 12px;
		color: rgba(255, 255, 255, 0.42);
		text-align: center;
	}

	.empty-icon {
		width: 38px;
		height: 38px;
		color: rgba(255, 255, 255, 0.08);
		margin-bottom: 4px;
	}

	.error-text {
		font-size: 12px;
		color: #f87171;
		text-align: center;
		max-width: 280px;
		line-height: 1.5;
	}

	.btn-retry {
		margin-top: 4px;
		padding: 5px 14px;
		font-size: 11.5px;
		color: #f87171;
		background: rgba(248, 113, 113, 0.08);
		border: 1px solid rgba(248, 113, 113, 0.22);
		border-radius: 6px;
		cursor: pointer;
		transition: background 0.12s;
	}

	.btn-retry:hover {
		background: rgba(248, 113, 113, 0.15);
	}

	/* Loading dots */
	.loading-dots {
		display: flex;
		gap: 5px;
		margin-bottom: 4px;
	}

	.loading-dots span {
		width: 5px;
		height: 5px;
		background: rgba(255, 255, 255, 0.18);
		border-radius: 50%;
		animation: dot-pulse 1.2s ease-in-out infinite;
	}

	.loading-dots span:nth-child(2) { animation-delay: 0.18s; }
	.loading-dots span:nth-child(3) { animation-delay: 0.36s; }

	@keyframes dot-pulse {
		0%, 60%, 100% { opacity: 0.18; transform: scale(1); }
		30% { opacity: 1; transform: scale(1.4); }
	}

	/* ── Article rows ────────────────────────────────────── */
	.art-row {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		padding: 9px 14px 9px 16px;
		border-left: 2px solid transparent;
		background: transparent;
		width: 100%;
		text-align: left;
		cursor: pointer;
		border-bottom: 1px solid rgba(255, 255, 255, 0.08);
		transition: background 0.1s ease, border-left-color 0.1s ease;
	}

	.art-row:hover {
		background: rgba(255, 255, 255, 0.04);
		border-left-color: #e11d48;
	}

	.art-info {
		flex: 1;
		min-width: 0;
	}

	.numart-badge {
		display: inline-block;
		font-family: ui-monospace, monospace;
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 0.07em;
		color: #fbbf24;
		background: rgba(251, 191, 36, 0.09);
		border: 1px solid rgba(251, 191, 36, 0.2);
		padding: 2px 6px;
		border-radius: 4px;
		line-height: 1.5;
	}

	.art-desc {
		font-size: 12px;
		color: rgba(255, 255, 255, 0.82);
		line-height: 1.35;
		margin: 3px 0 3px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.barcode-chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-family: ui-monospace, monospace;
		font-size: 9.5px;
		font-weight: 600;
		padding: 2px 6px;
		border-radius: 4px;
		border: 1px solid;
		line-height: 1.5;
	}

	.barcode-chip.has-code {
		color: #2dd4bf;
		background: rgba(45, 212, 191, 0.07);
		border-color: rgba(45, 212, 191, 0.18);
	}

	.barcode-chip.no-code {
		color: #f87171;
		background: rgba(248, 113, 113, 0.07);
		border-color: rgba(248, 113, 113, 0.18);
	}

	.add-btn {
		flex-shrink: 0;
		width: 20px;
		height: 20px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.08);
		color: rgba(255, 255, 255, 0.33);
		margin-top: 3px;
		transition: background 0.1s, color 0.1s;
	}

	.art-row:hover .add-btn {
		background: rgba(225, 29, 72, 0.15);
		color: #fb7185;
	}

	/* ── Load more ───────────────────────────────────────── */
	.btn-load-more {
		padding: 6px 16px;
		font-size: 11.5px;
		color: rgba(255, 255, 255, 0.35);
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.09);
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.12s;
	}

	.btn-load-more:hover:not(:disabled) {
		color: rgba(255, 255, 255, 0.65);
		border-color: rgba(255, 255, 255, 0.18);
	}

	.btn-load-more:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	/* ── Header action buttons ───────────────────────────── */
	.btn-clear {
		padding: 5px 11px;
		font-size: 11.5px;
		color: rgba(255, 255, 255, 0.50);
		background: transparent;
		border: 1px solid rgba(255, 255, 255, 0.09);
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.12s;
	}

	.btn-clear:hover {
		color: rgba(255, 255, 255, 0.6);
		border-color: rgba(255, 255, 255, 0.18);
	}

	.btn-print {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 14px;
		font-size: 12px;
		font-weight: 600;
		color: white;
		background: #e11d48;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		transition: background 0.15s, box-shadow 0.15s;
		font-family: 'Barlow', sans-serif;
		letter-spacing: 0.02em;
	}

	.btn-print:hover {
		background: #c81038;
		box-shadow: 0 0 18px rgba(225, 29, 72, 0.35);
	}

	/* ── Queue empty state ───────────────────────────────── */
	.queue-empty {
		gap: 6px;
	}

	.empty-printer-icon {
		width: 68px;
		height: 68px;
		color: rgba(255, 255, 255, 0.09);
		margin-bottom: 8px;
	}

	.empty-title {
		font-family: 'Barlow Condensed', sans-serif;
		font-size: 14px;
		font-weight: 700;
		color: rgba(255, 255, 255, 0.32);
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.empty-sub {
		font-size: 11.5px;
		color: rgba(255, 255, 255, 0.30);
		text-align: center;
		line-height: 1.65;
	}

	/* ── Queue cards ─────────────────────────────────────── */
	.queue-list {
		padding: 10px;
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.queue-card {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 11px;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: 8px;
		transition: border-color 0.12s;
	}

	.queue-card:hover {
		border-color: rgba(255, 255, 255, 0.22);
	}

	.queue-card-info {
		flex: 1;
		min-width: 0;
	}

	.queue-desc {
		font-size: 12px;
		color: rgba(255, 255, 255, 0.78);
		line-height: 1.35;
		margin: 3px 0 2px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.queue-code {
		font-family: ui-monospace, monospace;
		font-size: 9.5px;
		color: rgba(255, 255, 255, 0.40);
		margin-top: 1px;
	}

	/* ── Quantity stepper ────────────────────────────────── */
	.qty-stepper {
		display: flex;
		align-items: center;
		flex-shrink: 0;
		border: 1px solid rgba(255, 255, 255, 0.11);
		border-radius: 6px;
		overflow: hidden;
	}

	.qty-btn {
		width: 25px;
		height: 27px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(255, 255, 255, 0.04);
		border: none;
		color: rgba(255, 255, 255, 0.65);
		cursor: pointer;
		font-size: 15px;
		line-height: 1;
		transition: background 0.1s, color 0.1s;
	}

	.qty-btn:hover {
		background: rgba(255, 255, 255, 0.09);
		color: rgba(255, 255, 255, 0.85);
	}

	.qty-input {
		width: 34px;
		height: 27px;
		text-align: center;
		font-size: 12px;
		font-family: ui-monospace, monospace;
		font-weight: 600;
		color: rgba(255, 255, 255, 0.8);
		background: rgba(255, 255, 255, 0.03);
		border: none;
		border-left: 1px solid rgba(255, 255, 255, 0.09);
		border-right: 1px solid rgba(255, 255, 255, 0.09);
		outline: none;
		-moz-appearance: textfield;
	}

	.qty-input::-webkit-inner-spin-button,
	.qty-input::-webkit-outer-spin-button {
		-webkit-appearance: none;
	}

	/* ── Remove button ───────────────────────────────────── */
	.remove-btn {
		flex-shrink: 0;
		width: 25px;
		height: 25px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 6px;
		background: transparent;
		border: none;
		color: rgba(255, 255, 255, 0.32);
		cursor: pointer;
		transition: background 0.1s, color 0.1s;
	}

	.remove-btn:hover {
		background: rgba(248, 113, 113, 0.1);
		color: #f87171;
	}

</style>
