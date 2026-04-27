<script lang="ts">
	import { onMount } from 'svelte';
	import SkeletonList from '../components/SkeletonList.svelte';
	import ErrorBanner from '../components/ErrorBanner.svelte';
	import { nav } from '../lib/nav.svelte.js';
	import {
		getFraccionesInitData,
		getDbfPaths,
		saveFraccionPairing,
		deleteFraccionPairing,
		createEtiqueta,
		updateEtiqueta,
		deleteEtiqueta,
		setEmparejamientoEtiquetas
	} from '../lib/dbf.js';
	import type { ArticuloFracciones, ArticuloSearchResult, ArticuloPareado, FraccionRecord, Etiqueta } from '../lib/types.js';

	interface Props {
		onGoConfig: () => void;
	}
	let { onGoConfig }: Props = $props();

	// ── Paleta de colores para etiquetas ──────────────────────────
	const ETIQUETA_COLORS = [
		{ name: 'Azul',    hex: '#3b82f6' },
		{ name: 'Violeta', hex: '#8b5cf6' },
		{ name: 'Rosa',    hex: '#ec4899' },
		{ name: 'Rojo',    hex: '#ef4444' },
		{ name: 'Naranja', hex: '#f97316' },
		{ name: 'Ámbar',   hex: '#f59e0b' },
		{ name: 'Verde',   hex: '#22c55e' },
		{ name: 'Teal',    hex: '#14b8a6' },
	];

	function etiquetaBadgeStyle(color: string): string {
		return `color:${color}; background:${color}1a; border-color:${color}33;`;
	}

	// ── Datos ──────────────────────────────────────────────────────
	let articulos = $state<ArticuloFracciones[]>([]);
	let todosArticulos = $state<ArticuloSearchResult[]>([]);
	let etiquetas = $state<Etiqueta[]>([]);
	let query = $state('');
	let cargando = $state(false);
	let errorMsg = $state('');
	let sinCarpeta = $state(false);
	let expandedKeys = $state(new Set<string>());

	// ── Estado etiquetas ───────────────────────────────────────────
	let filtroEtiquetaId = $state<number | null>(null);

	// Popover de asignación
	let popoverEmp = $state<Emparejamiento | null>(null);
	let popoverX = $state(0);
	let popoverY = $state(0);

	// Modal de gestión
	let modalEtiquetasOpen = $state(false);
	let editingEtiqueta = $state<Etiqueta | null>(null);
	let formNombre = $state('');
	let formColor = $state('#3b82f6');
	let formError = $state('');
	let formSaving = $state(false);

	type Emparejamiento = {
		numart_origen: string;
		desc_origen: string;
		unidad_base: string;
		frac: FraccionRecord;
		pareado: ArticuloPareado;
	};

	const todosEmp = $derived<Emparejamiento[]>(
		articulos.flatMap((art) =>
			art.fracciones
				.filter((f) => f.pareado !== null)
				.map((f) => ({
					numart_origen: art.numart,
					desc_origen: art.desc,
					unidad_base: art.unidad_base,
					frac: f,
					pareado: f.pareado!
				}))
		)
	);

	const emparejamientos = $derived(
		todosEmp
			.filter((e) =>
				filtroEtiquetaId === null || e.frac.etiquetas.some((etq) => etq.id === filtroEtiquetaId)
			)
			.filter((e) =>
				!query.trim() ||
				matchTokens(
					query,
					e.desc_origen,
					e.numart_origen,
					e.pareado.desc,
					e.pareado.numart,
					e.frac.unidad,
					...e.frac.etiquetas.map((etq) => etq.nombre)
				)
			)
	);

	// ── Modal de nuevo emparejamiento (3 pasos) ───────────────────
	// Paso 0: cerrado
	// Paso 1: buscar artículo origen (client-side en `articulos`)
	// Paso 2: elegir fracción del artículo origen
	// Paso 3: buscar artículo destino (client-side en `todosArticulos`)
	let paso = $state<0 | 1 | 2 | 3>(0);
	let queryOrigen = $state('');
	let artOrigenActivo = $state<ArticuloFracciones | null>(null);
	let fraccionActiva = $state<{ numart: string; unidad: string } | null>(null);
	let queryDestino = $state('');

	const resultadosOrigen = $derived(
		queryOrigen.trim()
			? articulos.filter((a) => matchTokens(queryOrigen, a.desc, a.numart)).slice(0, 30)
			: []
	);

	const resultadosDestino = $derived(
		queryDestino.trim()
			? todosArticulos.filter((a) => matchTokens(queryDestino, a.desc, a.numart)).slice(0, 30)
			: []
	);

	// ── Carga ──────────────────────────────────────────────────────
	onMount(cargar);

	async function cargar() {
		cargando = true;
		errorMsg = '';
		sinCarpeta = false;
		articulos = [];
		todosArticulos = [];
		try {
			const paths = await getDbfPaths();
			if (!paths.dbf_arts || !paths.dbf_unidades) {
				sinCarpeta = true;
				return;
			}
			const data = await getFraccionesInitData();
			articulos = data.fracciones;
			todosArticulos = data.articulos;
			etiquetas = data.etiquetas;
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		} finally {
			cargando = false;
		}
	}

	// ── Búsqueda por tokens ───────────────────────────────────────
	function matchTokens(q: string, ...fields: string[]): boolean {
		const tokens = q.trim().toLowerCase().split(/\s+/).filter(Boolean);
		if (tokens.length === 0) return true;
		const haystack = fields.join(' ').toLowerCase();
		return tokens.every((t) => haystack.includes(t));
	}

	// ── Helpers visuales ──────────────────────────────────────────
	function fmt(v: number): string {
		return v.toLocaleString('es-MX', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	}

	type ConPrecios = { precio1: number; precio2: number; precio3: number; precio4: number; precio5: number };

	function getPrecio(obj: ConPrecios, n: number): number {
		const map: Record<number, number> = {
			1: obj.precio1, 2: obj.precio2, 3: obj.precio3, 4: obj.precio4, 5: obj.precio5
		};
		return map[n] ?? 0;
	}

	function pctDif(frac: number, par: number): { texto: string; clase: string } {
		if (Math.abs(frac) < 0.001) return { texto: '—', clase: 'text-slate-300' };
		const d = par - frac;
		if (Math.abs(d / frac) < 0.01) return { texto: '=', clase: 'text-slate-400' };
		const pct = (d / frac) * 100;
		return {
			texto: `${pct > 0 ? '+' : ''}${pct.toFixed(1)}%`,
			clase: pct > 0 ? 'text-red-500' : 'text-emerald-600'
		};
	}

	function difFmt(frac: number, par: number): { texto: string; clase: string } {
		const d = par - frac;
		if (Math.abs(d) < 0.001) return { texto: '=', clase: 'text-slate-400' };
		if (frac !== 0 && Math.abs(d / frac) < 0.01) return { texto: '=', clase: 'text-slate-400' };
		const pct = frac !== 0 ? ((d / frac) * 100).toFixed(1) : '—';
		const signo = d > 0 ? '+' : '';
		return {
			texto: `${signo}$${fmt(d)} (${signo}${pct}%)`,
			clase: d > 0 ? 'text-red-500' : 'text-emerald-600'
		};
	}

	// ── Acciones ──────────────────────────────────────────────────
	async function desvincular(numart_origen: string, unidad_fraccion: string) {
		try {
			await deleteFraccionPairing(numart_origen, unidad_fraccion);
			await cargar();
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		}
	}

	function toggleExpand(key: string) {
		const next = new Set(expandedKeys);
		if (next.has(key)) next.delete(key);
		else next.add(key);
		expandedKeys = next;
	}

	function focusOnMount(node: HTMLElement) {
		node.focus();
	}

	// ── Popover de etiquetas ──────────────────────────────────────
	function abrirPopover(e: Emparejamiento, anchor: HTMLElement) {
		const rect = anchor.getBoundingClientRect();
		popoverX = Math.min(rect.left, window.innerWidth - 272);
		popoverY = rect.bottom + 6;
		popoverEmp = e;
	}

	function cerrarPopover() {
		popoverEmp = null;
	}

	function clickFuera(node: HTMLElement, handler: () => void) {
		const listener = (ev: MouseEvent) => {
			if (!node.contains(ev.target as Node)) handler();
		};
		document.addEventListener('mousedown', listener, true);
		return { destroy() { document.removeEventListener('mousedown', listener, true); } };
	}

	async function toggleEtiqueta(emp: Emparejamiento, etiquetaId: number) {
		const ya = emp.frac.etiquetas.some((e) => e.id === etiquetaId);
		const nuevosIds = ya
			? emp.frac.etiquetas.filter((e) => e.id !== etiquetaId).map((e) => e.id)
			: [...emp.frac.etiquetas.map((e) => e.id), etiquetaId];

		// Actualización optimista
		const art = articulos.find((a) => a.numart === emp.numart_origen);
		if (art) {
			const frac = art.fracciones.find((f) => f.unidad === emp.frac.unidad);
			if (frac) {
				frac.etiquetas = ya
					? frac.etiquetas.filter((e) => e.id !== etiquetaId)
					: [...frac.etiquetas, etiquetas.find((e) => e.id === etiquetaId)!];
			}
		}
		articulos = [...articulos];

		try {
			await setEmparejamientoEtiquetas(emp.numart_origen, emp.frac.unidad, nuevosIds);
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
			await cargar();
		}
	}

	// ── Modal gestión de etiquetas ────────────────────────────────
	function abrirModalEtiquetas() {
		modalEtiquetasOpen = true;
		editingEtiqueta = null;
		formNombre = '';
		formColor = '#3b82f6';
		formError = '';
	}

	function cerrarModalEtiquetas() {
		modalEtiquetasOpen = false;
		editingEtiqueta = null;
		formError = '';
	}

	function empezarEditar(etq: Etiqueta) {
		editingEtiqueta = etq;
		formNombre = etq.nombre;
		formColor = etq.color;
		formError = '';
	}

	function cancelarEdicion() {
		editingEtiqueta = null;
		formNombre = '';
		formColor = '#3b82f6';
		formError = '';
	}

	async function guardarEtiqueta() {
		if (!formNombre.trim()) return;
		formSaving = true;
		formError = '';
		try {
			if (editingEtiqueta) {
				await updateEtiqueta(editingEtiqueta.id, formNombre.trim(), formColor);
				const id = editingEtiqueta.id;
				etiquetas = etiquetas.map((e) =>
					e.id === id ? { ...e, nombre: formNombre.trim(), color: formColor } : e
				);
				articulos = articulos.map((art) => ({
					...art,
					fracciones: art.fracciones.map((frac) => ({
						...frac,
						etiquetas: frac.etiquetas.map((etq) =>
							etq.id === id ? { ...etq, nombre: formNombre.trim(), color: formColor } : etq
						),
					})),
				}));
				editingEtiqueta = null;
			} else {
				const nueva = await createEtiqueta(formNombre.trim(), formColor);
				etiquetas = [...etiquetas, nueva];
			}
			formNombre = '';
			formColor = '#3b82f6';
		} catch (err) {
			formError = err instanceof Error ? err.message : String(err);
		} finally {
			formSaving = false;
		}
	}

	async function eliminarEtiqueta(id: number) {
		try {
			await deleteEtiqueta(id);
			etiquetas = etiquetas.filter((e) => e.id !== id);
			if (filtroEtiquetaId === id) filtroEtiquetaId = null;
			await cargar();
		} catch (err) {
			errorMsg = err instanceof Error ? err.message : String(err);
		}
	}

	function filtrarPorEtiqueta(id: number) {
		filtroEtiquetaId = filtroEtiquetaId === id ? null : id;
	}

	function handleKeyEtiqueta(ev: KeyboardEvent) {
		if (ev.key === 'Enter') guardarEtiqueta();
	}

	function handleKeyModalEtiquetas(ev: KeyboardEvent) {
		if (ev.key === 'Escape') cerrarModalEtiquetas();
	}

	function handleKeyPopover(ev: KeyboardEvent) {
		if (ev.key === 'Escape') cerrarPopover();
	}

	// ── Modal de nuevo emparejamiento ─────────────────────────────
	function abrirModal() {
		paso = 1;
		queryOrigen = '';
		artOrigenActivo = null;
		fraccionActiva = null;
		queryDestino = '';
	}

	function cerrarModal() {
		paso = 0;
		queryOrigen = '';
		artOrigenActivo = null;
		fraccionActiva = null;
		queryDestino = '';
	}

	function seleccionarOrigen(art: ArticuloFracciones) {
		artOrigenActivo = art;
		paso = 2;
	}

	function seleccionarFraccion(numart: string, unidad: string) {
		fraccionActiva = { numart, unidad };
		queryDestino = '';
		paso = 3;
	}

	function onQueryOrigenInput(e: Event) {
		queryOrigen = (e.target as HTMLInputElement).value.toUpperCase();
		(e.target as HTMLInputElement).value = queryOrigen;
	}

	function onQueryDestinoInput(e: Event) {
		const input = e.target as HTMLInputElement;
		const upper = input.value.toUpperCase();
		input.value = upper;
		queryDestino = upper;
	}

	async function seleccionarDestino(destino: ArticuloSearchResult) {
		if (!fraccionActiva) return;
		try {
			await saveFraccionPairing(fraccionActiva.numart, fraccionActiva.unidad, destino.numart);
			cerrarModal();
			await cargar();
		} catch (e) {
			errorMsg = e instanceof Error ? e.message : String(e);
		}
	}
</script>

<div class="min-h-screen bg-bg">
	<!-- ── Header ──────────────────────────────────────────────── -->
	<div class="bg-[#0f1f38] px-4 pt-5 pb-4 md:px-6">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-3">
				<button
					class="md:hidden w-9 h-9 flex items-center justify-center rounded-lg text-white/60
						hover:bg-white/10 active:bg-white/20 transition-colors"
					onclick={() => nav.toggle()}
					aria-label="Abrir menú"
				>
					<svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">
						<line x1="3" y1="6" x2="21" y2="6" />
						<line x1="3" y1="12" x2="21" y2="12" />
						<line x1="3" y1="18" x2="21" y2="18" />
					</svg>
				</button>
				<div>
					<p class="text-[11px] font-semibold tracking-[0.12em] uppercase text-white/40">Inventario</p>
					<h1 class="font-barlow-condensed text-[22px] font-bold text-white leading-none">
						Verificador de Fracciones
					</h1>
				</div>
			</div>

			<div class="flex items-center gap-2">
				<!-- Gestionar etiquetas -->
				{#if !sinCarpeta && !cargando}
					<button
						onclick={abrirModalEtiquetas}
						class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-white/10 border border-white/15
							text-white text-[12px] font-medium hover:bg-white/20 active:bg-white/25 transition-colors"
					>
						<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<path d="M20.59 13.41l-7.17 7.17a2 2 0 01-2.83 0L2 12V2h10l8.59 8.59a2 2 0 010 2.82z"/>
							<line x1="7" y1="7" x2="7.01" y2="7"/>
						</svg>
						Etiquetas
					</button>
				{/if}
				<!-- Nuevo emparejamiento -->
				{#if !sinCarpeta && !cargando}
					<button
						onclick={abrirModal}
						class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-white/10 border border-white/15
							text-white text-[12px] font-medium hover:bg-white/20 active:bg-white/25 transition-colors"
					>
						<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
							<line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
						</svg>
						Nuevo
					</button>
				{/if}
				<!-- Recargar -->
				<button
					onclick={cargar}
					disabled={cargando}
					class="w-9 h-9 flex items-center justify-center rounded-lg text-white/60
						hover:bg-white/10 active:bg-white/20 transition-colors disabled:opacity-40"
					title="Releer archivos DBF"
				>
					<svg class="w-4 h-4 {cargando ? 'animate-spin-fast' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
						<polyline points="23 4 23 10 17 10" />
						<path d="M20.49 15a9 9 0 11-2.12-9.36L23 10" />
					</svg>
				</button>
			</div>
		</div>

		<!-- Barra de búsqueda -->
		{#if !sinCarpeta && !cargando && todosEmp.length > 0}
			<div class="mt-4 relative">
				<svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-white/30 pointer-events-none"
					viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">
					<circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
				</svg>
				<input
					type="text"
					placeholder="Buscar por artículo, fracción, clave o etiqueta…"
					bind:value={query}
					class="w-full h-9 pl-9 pr-3 rounded-lg text-[13px] font-barlow
						bg-white/[0.08] border border-white/10 text-white placeholder:text-white/30
						focus:outline-none focus:ring-1 focus:ring-white/20 focus:bg-white/[0.12] transition-colors"
				/>
			</div>
			<!-- Pastillas de filtro por etiqueta -->
			{#if etiquetas.length > 0}
				<div class="mt-2 flex items-center gap-1.5 flex-wrap">
					<span class="text-[10px] font-semibold uppercase tracking-wider text-white/30 mr-0.5">Filtrar:</span>
					<button
						onclick={() => filtroEtiquetaId = null}
						class="h-6 px-2.5 rounded-full text-[11px] font-medium transition-colors
							{filtroEtiquetaId === null ? 'bg-white/20 text-white' : 'text-white/50 hover:bg-white/10'}"
					>Todos</button>
					{#each etiquetas as etq (etq.id)}
						<button
							onclick={() => filtrarPorEtiqueta(etq.id)}
							style={filtroEtiquetaId === etq.id ? `background:${etq.color}33; color:${etq.color}; border-color:${etq.color}66;` : ''}
							class="h-6 px-2.5 rounded-full text-[11px] font-medium border transition-colors
								{filtroEtiquetaId === etq.id ? 'border' : 'text-white/50 border-white/20 hover:bg-white/10'}"
						>{etq.nombre}</button>
					{/each}
				</div>
			{/if}
		{/if}
	</div>

	<!-- ── Contenido ───────────────────────────────────────────── -->
	<div class="px-4 py-4 md:px-6">

		{#if errorMsg}
			<ErrorBanner message={errorMsg} />
		{/if}

		{#if sinCarpeta}
			<div class="mt-8 flex flex-col items-center text-center animate-fadeSlide">
				<div class="w-14 h-14 rounded-full bg-amber/10 flex items-center justify-center mb-4">
					<svg class="w-7 h-7 text-amber" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
						<path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
					</svg>
				</div>
				<h2 class="font-barlow-condensed text-[18px] font-bold text-slate-700 mb-1">Archivos DBF no configurados</h2>
				<p class="text-[13px] text-slate-400 mb-5 max-w-xs">
					Configura el archivo de artículos y el de fracciones en Configuración.
				</p>
				<button
					onclick={onGoConfig}
					class="h-9 px-5 rounded-lg bg-navy text-white text-[13px] font-medium font-barlow
						hover:opacity-90 active:opacity-80 transition-opacity"
				>
					Ir a Configuración
				</button>
			</div>

		{:else if cargando}
			<SkeletonList count={4} />

		{:else if todosEmp.length === 0}
			<!-- Sin emparejamientos aún -->
			<div class="mt-12 flex flex-col items-center text-center animate-fadeSlide">
				<div class="w-14 h-14 rounded-full bg-navy/8 flex items-center justify-center mb-4">
					<svg class="w-7 h-7 text-navy/40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
						<path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71" />
						<path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71" />
					</svg>
				</div>
				<h2 class="font-barlow-condensed text-[18px] font-bold text-slate-600 mb-1">Sin emparejamientos</h2>
				<p class="text-[13px] text-slate-400 mb-5 max-w-xs">
					Crea un emparejamiento para comparar el precio de una fracción con su artículo equivalente.
				</p>
				<button
					onclick={abrirModal}
					class="h-9 px-5 rounded-lg bg-navy text-white text-[13px] font-medium font-barlow
						hover:opacity-90 active:opacity-80 transition-opacity flex items-center gap-2"
				>
					<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
						<line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
					</svg>
					Nuevo emparejamiento
				</button>
			</div>

		{:else if emparejamientos.length === 0}
			<div class="mt-10 text-center animate-fadeSlide">
				<p class="text-[14px] text-slate-400">Sin resultados para "{query}"</p>
			</div>

		{:else}
			<p class="text-[11px] text-slate-400 mb-3 font-medium tracking-wide">
				{emparejamientos.length} emparejamiento{emparejamientos.length !== 1 ? 's' : ''}
				{query.trim() ? `· filtrado de ${todosEmp.length}` : ''}
			</p>

			<div class="bg-surface rounded-card shadow-card overflow-hidden">
				<table class="w-full border-collapse">
					<thead>
						<tr class="bg-bg/70 border-b border-slate-100">
							<th class="text-left px-4 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[22%]">Clave origen</th>
							<th class="text-center px-2 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[6%]">Base</th>
							<th class="text-center px-2 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[6%]">Fracc.</th>
							<th class="text-left px-4 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[20%]">Clave pareado</th>
							<th class="text-center px-2 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[6%]">Unidad</th>
							<th class="text-left px-3 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[22%]">Etiquetas</th>
							<th class="hidden sm:table-cell text-right px-3 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[12%]">Dif. %</th>
							<th class="w-[6%]"></th>
						</tr>
					</thead>
					<tbody>
						{#each emparejamientos as e (`${e.numart_origen}|${e.frac.unidad}`)}
							{@const key = `${e.numart_origen}|${e.frac.unidad}`}
							{@const isOpen = expandedKeys.has(key)}
	
							<!-- Fila resumen -->
							<tr
								class="border-t border-slate-100 hover:bg-bg/50 transition-colors cursor-pointer group"
								onclick={() => toggleExpand(key)}
							>
								<td class="px-4 py-2.5 min-w-0">
									<p class="font-mono text-[13px] font-bold text-navy leading-snug truncate">{e.numart_origen}</p>
									<p class="text-[11px] text-slate-400 leading-none mt-0.5 truncate">{e.desc_origen}</p>
								</td>
								<td class="px-2 py-2.5 text-center">
									<span class="inline-block text-[11px] font-semibold text-slate-600 bg-slate-100 border border-slate-200 px-1.5 py-0.5 rounded font-mono leading-none whitespace-nowrap">{e.unidad_base}</span>
								</td>
								<td class="px-2 py-2.5 text-center">
									<span class="inline-block text-[11px] font-semibold text-amber bg-amber/10 border border-amber/20 px-1.5 py-0.5 rounded font-mono leading-none whitespace-nowrap">{e.frac.unidad}</span>
								</td>
								<td class="px-4 py-2.5 min-w-0">
									<p class="font-mono text-[13px] font-bold text-slate-600 leading-snug truncate">{e.pareado.numart}</p>
									<p class="text-[11px] text-slate-400 leading-none mt-0.5 truncate">{e.pareado.desc}</p>
								</td>
								<td class="px-2 py-2.5 text-center">
									<span class="inline-block text-[11px] font-semibold text-slate-600 bg-slate-100 border border-slate-200 px-1.5 py-0.5 rounded font-mono leading-none whitespace-nowrap">{e.pareado.unidad}</span>
								</td>
								<!-- Columna Etiquetas -->
								<td class="px-3 py-2.5 w-[22%]">
									<div class="flex flex-wrap gap-1 items-center">
										{#each e.frac.etiquetas as etq (etq.id)}
											<button
												onclick={(ev) => { ev.stopPropagation(); filtrarPorEtiqueta(etq.id); }}
												style={etiquetaBadgeStyle(etq.color)}
												class="inline-block text-[11px] font-semibold px-1.5 py-0.5 rounded border font-mono leading-none whitespace-nowrap hover:opacity-70 transition-opacity"
											>{etq.nombre}</button>
										{/each}
										<button
											onclick={(ev) => { ev.stopPropagation(); abrirPopover(e, ev.currentTarget as HTMLElement); }}
											class="w-5 h-5 flex items-center justify-center rounded-full text-slate-400
												hover:bg-slate-100 hover:text-slate-600 transition-colors flex-shrink-0"
											title="Asignar etiquetas"
										>
											<svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
												<line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
											</svg>
										</button>
									</div>
								</td>
								<td class="hidden sm:table-cell px-3 py-2.5">
									<div class="flex flex-wrap gap-x-2 gap-y-0.5 justify-end">
										{#each [1, 2, 3, 4, 5] as n}
											{@const p = pctDif(getPrecio(e.frac, n), getPrecio(e.pareado, n))}
											<span class="font-mono text-[11px] font-semibold {p.clase} whitespace-nowrap">
												<span class="text-slate-300 font-normal text-[10px]">P{n} </span>{p.texto}
											</span>
										{/each}
									</div>
								</td>
								<td class="px-3 py-2.5 text-right">
									<div class="flex items-center justify-end gap-1">
										<button
											onclick={(ev) => { ev.stopPropagation(); desvincular(e.numart_origen, e.frac.unidad); }}
											class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400
												hover:text-red-500 hover:bg-red-50 transition-colors"
											title="Desvincular"
										>
											<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
												<path d="M18.36 6.64a9 9 0 11-12.73 0" /><line x1="12" y1="2" x2="12" y2="12" />
											</svg>
										</button>
										<div class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400 group-hover:text-slate-600 transition-colors">
											<svg class="w-4 h-4 transition-transform duration-200 {isOpen ? 'rotate-180' : 'rotate-0'}"
												viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
												<polyline points="6 9 12 15 18 9" />
											</svg>
										</div>
									</div>
								</td>
							</tr>

							<!-- Fila detalle (expandida) -->
							{#if isOpen}
								<tr class="border-t border-slate-100">
									<td colspan="8" class="px-0 py-0 bg-bg/40">
										<table class="w-full text-[12px]">
											<thead>
												<tr class="bg-bg/60">
													<th class="text-left px-6 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px] w-12">Nivel</th>
													<th class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]">Fracción <span class="font-mono normal-case">({e.frac.unidad})</span></th>
													<th class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]">Artículo pareado</th>
													<th class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]">Diferencia</th>
												</tr>
											</thead>
											<tbody>
												{#each [1, 2, 3, 4, 5] as n}
													{@const pFrac = getPrecio(e.frac, n)}
													{@const pPar = getPrecio(e.pareado, n)}
													{@const dif = difFmt(pFrac, pPar)}
													<tr class="border-t border-slate-100 hover:bg-bg/40 transition-colors">
														<td class="px-6 py-2.5 font-semibold text-slate-500 text-[13px]">P{n}</td>
														<td class="px-4 py-2.5 text-right font-mono text-slate-700">${fmt(pFrac)}</td>
														<td class="px-4 py-2.5 text-right font-mono text-slate-700">${fmt(pPar)}</td>
														<td class="px-4 py-2.5 text-right font-mono font-semibold {dif.clase}">{dif.texto}</td>
													</tr>
												{/each}
											</tbody>
										</table>
									</td>
								</tr>
							{/if}
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>
</div>

<!-- ── Popover de etiquetas ─────────────────────────────────────── -->
{#if popoverEmp !== null}
	{@const popEmp = popoverEmp}
	<div
		class="fixed z-40 bg-white rounded-xl shadow-2xl border border-slate-100 w-64 p-3"
		style="top:{popoverY}px; left:{popoverX}px;"
		use:clickFuera={cerrarPopover}
		onkeydown={handleKeyPopover}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<p class="text-[10px] font-semibold uppercase tracking-wider text-slate-400 mb-2">
			Etiquetas · <span class="font-mono normal-case">{popEmp.frac.unidad}</span>
		</p>
		{#if etiquetas.length === 0}
			<p class="text-[12px] text-slate-400 text-center py-2">Sin etiquetas creadas</p>
		{:else}
			<div class="flex flex-col gap-0.5">
				{#each etiquetas as etq (etq.id)}
					{@const activa = popEmp.frac.etiquetas.some((e) => e.id === etq.id)}
					<button
						onclick={() => toggleEtiqueta(popEmp, etq.id)}
						class="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-slate-50 transition-colors text-left"
					>
						<div class="w-4 h-4 rounded border-2 flex items-center justify-center flex-shrink-0 transition-colors
							{activa ? 'border-navy bg-navy' : 'border-slate-300'}">
							{#if activa}
								<svg class="w-2.5 h-2.5 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
									<polyline points="20 6 9 17 4 12" />
								</svg>
							{/if}
						</div>
						<span
							style={etiquetaBadgeStyle(etq.color)}
							class="text-[11px] font-semibold px-1.5 py-0.5 rounded border font-mono leading-none"
						>{etq.nombre}</span>
					</button>
				{/each}
			</div>
		{/if}
		<div class="mt-2 pt-2 border-t border-slate-100">
			<button
				onclick={() => { cerrarPopover(); abrirModalEtiquetas(); }}
				class="text-[11px] text-navy font-medium hover:underline"
			>+ Gestionar etiquetas</button>
		</div>
	</div>
{/if}

<!-- ── Modal de gestión de etiquetas ───────────────────────────── -->
{#if modalEtiquetasOpen}
	<div
		class="fixed inset-0 bg-black/40 z-50 flex items-end sm:items-center justify-center p-4"
		onclick={cerrarModalEtiquetas}
		onkeydown={handleKeyModalEtiquetas}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<div
			class="bg-white rounded-xl shadow-2xl w-full max-w-sm max-h-[80vh] flex flex-col overflow-hidden"
			onclick={(ev) => ev.stopPropagation()}
			onkeydown={(ev) => ev.stopPropagation()}
			role="document"
		>
			<!-- Header -->
			<div class="px-4 pt-4 pb-3 border-b border-slate-100 flex items-center justify-between">
				<div>
					<p class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Configuración</p>
					<h2 class="font-barlow-condensed text-[17px] font-bold text-navy">Gestionar etiquetas</h2>
				</div>
				<button
					onclick={cerrarModalEtiquetas}
					aria-label="Cerrar"
					class="w-8 h-8 flex items-center justify-center rounded-lg text-slate-400
						hover:bg-slate-100 hover:text-slate-600 transition-colors"
				>
					<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
						<line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
					</svg>
				</button>
			</div>

			<!-- Lista de etiquetas existentes -->
			<div class="overflow-y-auto flex-1 divide-y divide-slate-50">
				{#if etiquetas.length === 0}
					<p class="text-center text-[13px] text-slate-400 py-8">Sin etiquetas creadas aún</p>
				{:else}
					{#each etiquetas as etq (etq.id)}
						<div class="px-4 py-2.5 flex items-center gap-3">
							<span
								style={etiquetaBadgeStyle(etq.color)}
								class="text-[11px] font-semibold px-1.5 py-0.5 rounded border font-mono leading-none flex-shrink-0"
							>{etq.nombre}</span>
							<span class="flex-1"></span>
							<button
								onclick={() => empezarEditar(etq)}
								class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400
									hover:text-navy hover:bg-slate-100 transition-colors"
								title="Editar"
							>
								<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
									<path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/>
									<path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
								</svg>
							</button>
							<button
								onclick={() => eliminarEtiqueta(etq.id)}
								class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400
									hover:text-red-500 hover:bg-red-50 transition-colors"
								title="Eliminar"
							>
								<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
									<polyline points="3 6 5 6 21 6"/>
									<path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/>
									<path d="M10 11v6M14 11v6"/>
									<path d="M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2"/>
								</svg>
							</button>
						</div>
					{/each}
				{/if}
			</div>

			<!-- Formulario crear / editar -->
			<div class="px-4 py-3 border-t border-slate-100">
				<div class="flex items-center justify-between mb-2">
					<p class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
						{editingEtiqueta ? 'Editar etiqueta' : 'Nueva etiqueta'}
					</p>
					{#if editingEtiqueta}
						<button
							onclick={cancelarEdicion}
							class="text-[11px] text-slate-400 hover:text-slate-600 transition-colors"
						>Cancelar</button>
					{/if}
				</div>
				{#if formError}
					<p class="text-[11px] text-red-500 mb-2">{formError}</p>
				{/if}
				<div class="flex gap-2 items-start">
					<input
						type="text"
						bind:value={formNombre}
						placeholder="Nombre…"
						maxlength="24"
						onkeydown={handleKeyEtiqueta}
						class="flex-1 h-8 px-2 text-[13px] border border-slate-200 rounded-lg
							focus:outline-none focus:ring-1 focus:ring-navy/30 focus:border-navy/40 transition-colors"
					/>
					<button
						onclick={guardarEtiqueta}
						disabled={formSaving || !formNombre.trim()}
						class="h-8 px-3 rounded-lg bg-navy text-white text-[12px] font-medium
							hover:opacity-90 active:opacity-80 disabled:opacity-40 transition-opacity whitespace-nowrap"
					>{editingEtiqueta ? 'Guardar' : 'Crear'}</button>
				</div>
				<!-- Paleta de colores -->
				<div class="flex gap-1.5 mt-2 flex-wrap">
					{#each ETIQUETA_COLORS as c}
						<button
							onclick={() => formColor = c.hex}
							title={c.name}
							class="w-5 h-5 rounded-full border-2 transition-all flex-shrink-0
								{formColor === c.hex ? 'border-slate-600 scale-125' : 'border-transparent hover:scale-110'}"
							style="background:{c.hex};"
						></button>
					{/each}
				</div>
				<!-- Vista previa -->
				{#if formNombre.trim()}
					<div class="mt-2 flex items-center gap-2">
						<span class="text-[10px] text-slate-400 uppercase tracking-wider">Vista previa:</span>
						<span
							style={etiquetaBadgeStyle(formColor)}
							class="inline-block text-[11px] font-semibold px-1.5 py-0.5 rounded border font-mono leading-none"
						>{formNombre}</span>
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<!-- ── Modal de nuevo emparejamiento ───────────────────────────── -->
{#if paso > 0}
	<div
		class="fixed inset-0 bg-black/40 z-50 flex items-end sm:items-center justify-center p-4"
		onclick={cerrarModal}
		onkeydown={(e) => e.key === 'Escape' && cerrarModal()}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<div
			class="bg-white rounded-xl shadow-2xl w-full max-w-md max-h-[80vh] flex flex-col overflow-hidden"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="document"
		>
			<!-- Header del modal -->
			<div class="px-4 pt-4 pb-3 border-b border-slate-100">
				<div class="flex items-center justify-between mb-2">
					<div class="flex items-center gap-2">
						<!-- Indicador de pasos -->
						{#each [1, 2, 3] as s}
							<div class="flex items-center gap-1">
								<div class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold
									{paso >= s ? 'bg-navy text-white' : 'bg-slate-100 text-slate-400'}">
									{s}
								</div>
								{#if s < 3}
									<div class="w-4 h-px {paso > s ? 'bg-navy' : 'bg-slate-200'}"></div>
								{/if}
							</div>
						{/each}
					</div>
					<button
						onclick={cerrarModal}
						aria-label="Cerrar"
						class="w-8 h-8 flex items-center justify-center rounded-lg text-slate-400
							hover:bg-slate-100 hover:text-slate-600 transition-colors"
					>
						<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
							<line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
						</svg>
					</button>
				</div>

				{#if paso === 1}
					<p class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Paso 1 de 3</p>
					<p class="font-barlow-condensed text-[17px] font-bold text-navy">Buscar artículo origen</p>
					<div class="relative mt-3">
						<svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none"
							viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">
							<circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
						</svg>
						<input
							type="text"
							placeholder="Descripción o clave del artículo con fracciones…"
							value={queryOrigen}
							oninput={onQueryOrigenInput}
							use:focusOnMount
							class="w-full h-9 pl-9 pr-3 rounded-lg text-[13px] border border-slate-200 uppercase
								focus:outline-none focus:ring-2 focus:ring-navy/20 focus:border-navy/40 transition-colors"
						/>
					</div>

				{:else if paso === 2}
					<p class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Paso 2 de 3</p>
					<p class="font-barlow-condensed text-[17px] font-bold text-navy">Elegir fracción</p>
					{#if artOrigenActivo}
						<p class="text-[12px] text-slate-500 mt-0.5 truncate">{artOrigenActivo.desc}</p>
					{/if}

				{:else if paso === 3}
					<p class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">Paso 3 de 3</p>
					<p class="font-barlow-condensed text-[17px] font-bold text-navy">
						Buscar artículo destino
						{#if fraccionActiva}<span class="text-slate-400 font-normal text-[14px]">· fracc. {fraccionActiva.unidad}</span>{/if}
					</p>
					<div class="relative mt-3">
						<svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none"
							viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round">
							<circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
						</svg>
						<input
							type="text"
							placeholder="Descripción o clave del artículo a comparar…"
							value={queryDestino}
							oninput={onQueryDestinoInput}
							use:focusOnMount
							class="w-full h-9 pl-9 pr-3 rounded-lg text-[13px] border border-slate-200 uppercase
								focus:outline-none focus:ring-2 focus:ring-navy/20 focus:border-navy/40 transition-colors"
						/>
					</div>
				{/if}
			</div>

			<!-- Cuerpo del modal -->
			<div class="overflow-y-auto flex-1">
				{#if paso === 1}
					{#if !queryOrigen.trim()}
						<p class="text-center text-[13px] text-slate-400 py-8">Escribe para buscar</p>
					{:else if resultadosOrigen.length === 0}
						<p class="text-center text-[13px] text-slate-400 py-8">Sin resultados para "{queryOrigen}"</p>
					{:else}
						{#each resultadosOrigen as art (art.numart)}
							<button
								onclick={() => seleccionarOrigen(art)}
								class="w-full text-left px-4 py-3 border-b border-slate-50 hover:bg-sky-50 transition-colors"
							>
								<p class="text-[13px] font-semibold text-navy leading-snug">{art.desc}</p>
								<div class="flex items-center gap-2 mt-0.5">
									<span class="font-mono text-[11px] text-slate-400">{art.numart}</span>
									<span class="text-slate-300">·</span>
									<span class="text-[11px] text-slate-500">{art.fracciones.length} fracc.</span>
								</div>
							</button>
						{/each}
					{/if}

				{:else if paso === 2 && artOrigenActivo}
					{#each artOrigenActivo.fracciones as frac (frac.unidad)}
						{@const yaPareada = frac.pareado !== null}
						<button
							onclick={() => seleccionarFraccion(artOrigenActivo!.numart, frac.unidad)}
							class="w-full text-left px-4 py-3 border-b border-slate-50 hover:bg-sky-50 transition-colors"
						>
							<div class="flex items-center justify-between gap-2">
								<span class="font-mono font-bold text-navy text-[14px]">{frac.unidad}</span>
								{#if yaPareada}
									<span class="text-[10px] font-semibold text-amber/80 bg-amber/10 border border-amber/20 px-1.5 py-0.5 rounded">
										Ya emparejada
									</span>
								{/if}
							</div>
							{#if yaPareada && frac.pareado}
								<p class="text-[11px] text-slate-400 mt-0.5 truncate">→ {frac.pareado.desc}</p>
							{/if}
						</button>
					{/each}

				{:else if paso === 3}
					{#if !queryDestino.trim()}
						<p class="text-center text-[13px] text-slate-400 py-8">Escribe para buscar</p>
					{:else if resultadosDestino.length === 0}
						<p class="text-center text-[13px] text-slate-400 py-8">Sin resultados para "{queryDestino}"</p>
					{:else}
						{#each resultadosDestino as art (art.numart)}
							<button
								onclick={() => seleccionarDestino(art)}
								class="w-full text-left px-4 py-3 border-b border-slate-50 hover:bg-sky-50 transition-colors"
							>
								<p class="text-[13px] font-semibold text-navy leading-snug">{art.desc}</p>
								<div class="flex items-center gap-2 mt-0.5">
									<span class="font-mono text-[11px] text-slate-400">{art.numart}</span>
									{#if art.unidad}
										<span class="text-slate-300">·</span>
										<span class="text-[11px] text-slate-500">{art.unidad}</span>
									{/if}
								</div>
							</button>
						{/each}
					{/if}
				{/if}
			</div>

			<!-- Pie: botón volver -->
			{#if paso > 1}
				<div class="px-4 py-3 border-t border-slate-100">
					<button
						onclick={() => { paso = (paso - 1) as 1 | 2; }}
						class="text-[12px] font-medium text-slate-500 hover:text-slate-700 flex items-center gap-1 transition-colors"
					>
						<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<polyline points="15 18 9 12 15 6" />
						</svg>
						Volver
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}
