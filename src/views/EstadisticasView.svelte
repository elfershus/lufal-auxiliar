<script lang="ts">
	import { onDestroy } from 'svelte';
	import {
		Chart, BarController, BarElement,
		CategoryScale, LinearScale, Tooltip, Legend
	} from 'chart.js';
	import { getEstadisticasDosAnios } from '../lib/dbf.js';
	import { appConfig } from '../lib/config.svelte.js';
	import { formatMXN } from '../lib/utils.js';
	import type { EstadisticasResult, PeriodoStat } from '../lib/types.js';

	Chart.register(BarController, BarElement, CategoryScale, LinearScale, Tooltip, Legend);

	interface Props {
		onGoConfig: () => void;
	}
	let { onGoConfig }: Props = $props();

	const MESES_COMPLETOS = ['Enero','Febrero','Marzo','Abril','Mayo','Junio','Julio','Agosto','Septiembre','Octubre','Noviembre','Diciembre'];
	const MESES_CORTOS = ['Ene','Feb','Mar','Abr','May','Jun','Jul','Ago','Sep','Oct','Nov','Dic'];

	const hoy = new Date();
	const anioActual = hoy.getFullYear();
	const mesActual = hoy.getMonth();

	let cargando       = $state(true);
	let errorMsg       = $state('');
	let sinArchivo     = $state(false);
	let datosActual    = $state<EstadisticasResult | null>(null);
	let datosAnioAnterior = $state<EstadisticasResult | null>(null);
	let canvasMes      = $state<HTMLCanvasElement | null>(null);
	let canvasAnual    = $state<HTMLCanvasElement | null>(null);
	let chartMes: Chart | null = null;
	let chartAnual: Chart | null = null;

	// Tab y navegación de mes
	let tabActiva       = $state<'detalles' | 'vision'>('detalles');
	let mesSeleccionado = $state(mesActual);
	let mesCambiando    = $state(false);

	function getPeriodo(datos: EstadisticasResult | null, anio: number, mesIdx: number): PeriodoStat {
		const key = `${anio}-${String(mesIdx + 1).padStart(2, '0')}`;
		return datos?.periodos.find(p => p.periodo === key) ?? {
			periodo: key,
			ventas_importe: 0, compras_importe: 0,
			ventas_count: 0, compras_count: 0,
			facturas_importe: 0, facturas_count: 0,
			remisiones_importe: 0, remisiones_count: 0,
			notas_importe: 0, notas_count: 0,
			credito_importe: 0, credito_count: 0,
			abonos_importe: 0, abonos_count: 0,
		};
	}

	function deltaPct(actual: number, anterior: number): number | null {
		if (anterior === 0) return null;
		return ((actual - anterior) / anterior) * 100;
	}

	function navMes(dir: -1 | 1) {
		const siguiente = mesSeleccionado + dir;
		if (siguiente < 0 || siguiente > mesActual) return;
		mesCambiando = true;
		setTimeout(() => {
			mesSeleccionado = siguiente;
			mesCambiando = false;
		}, 110);
	}

	async function cargar() {
		cargando = true;
		errorMsg = '';
		sinArchivo = false;
		datosActual = null;
		datosAnioAnterior = null;

		try {
			const resultado = await getEstadisticasDosAnios(anioActual, appConfig.numalm || undefined);
			datosActual = resultado.actual;
			datosAnioAnterior = resultado.anterior;
		} catch (e) {
			const msg = e instanceof Error ? e.message : String(e);
			if (msg.includes('no configurado')) sinArchivo = true;
			else errorMsg = msg;
		}
		cargando = false;
	}

	$effect(() => { cargar(); });

	// Derived para tab Detalles (reactive a mesSeleccionado)
	const mesData     = $derived(getPeriodo(datosActual,       anioActual,     mesSeleccionado));
	const mesPrevData = $derived(getPeriodo(datosAnioAnterior, anioActual - 1, mesSeleccionado));
	const deltaVentas  = $derived(deltaPct(mesData.ventas_importe,  mesPrevData.ventas_importe));
	const deltaCompras = $derived(deltaPct(mesData.compras_importe, mesPrevData.compras_importe));
	const deltaAbonos  = $derived(deltaPct(mesData.abonos_importe,  mesPrevData.abonos_importe));
	const balanceActual = $derived(mesData.ventas_importe + mesData.abonos_importe - mesData.compras_importe);
	const balancePrevio = $derived(mesPrevData.ventas_importe + mesPrevData.abonos_importe - mesPrevData.compras_importe);
	const deltaBalance  = $derived(deltaPct(balanceActual, balancePrevio));

	// Derived para tab Visión General
	const filasMeses = $derived(
		MESES_CORTOS.map((_, i) => {
			const act = getPeriodo(datosActual,       anioActual,     i);
			const ant = getPeriodo(datosAnioAnterior, anioActual - 1, i);
			return {
				idx: i,
				nombre: MESES_COMPLETOS[i],
				act, ant,
				deltaV: deltaPct(act.ventas_importe,  ant.ventas_importe),
				deltaC: deltaPct(act.compras_importe, ant.compras_importe),
				deltaA: deltaPct(act.abonos_importe,  ant.abonos_importe),
			};
		})
	);

	// Totales anuales para Visión General (ya existentes en el tipo)
	const balAnual      = $derived(datosActual ? datosActual.total_ventas + datosActual.total_abonos - datosActual.total_compras : 0);
	const balAnualAnt   = $derived(datosAnioAnterior ? datosAnioAnterior.total_ventas + datosAnioAnterior.total_abonos - datosAnioAnterior.total_compras : 0);
	const deltaBalAnual = $derived(deltaPct(balAnual, balAnualAnt));

	// Chart mensual: redibujar al cambiar mesSeleccionado
	$effect(() => {
		if (!datosActual || !canvasMes) return;
		void mesSeleccionado;
			const c = new Chart(canvasMes, {
				type: 'bar',
				data: {
					labels: ['Contado', 'Compras', 'Abonos'],
					datasets: [
						{
							label: String(anioActual),
							data: [mesData.ventas_importe, mesData.compras_importe, mesData.abonos_importe],
							backgroundColor: ['rgba(31,146,84,0.85)', 'rgba(232,130,10,0.85)', 'rgba(139,92,246,0.85)'],
							borderColor: ['rgba(31,146,84,1)', 'rgba(232,130,10,1)', 'rgba(139,92,246,1)'],
							borderWidth: 1,
							borderRadius: 4,
						},
						{
							label: String(anioActual - 1),
							data: [mesPrevData.ventas_importe, mesPrevData.compras_importe, mesPrevData.abonos_importe],
							backgroundColor: ['rgba(31,146,84,0.25)', 'rgba(232,130,10,0.25)', 'rgba(139,92,246,0.25)'],
							borderColor: ['rgba(31,146,84,0.45)', 'rgba(232,130,10,0.45)', 'rgba(139,92,246,0.45)'],
							borderWidth: 1,
							borderRadius: 4,
						},
					],
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: {
							display: true, position: 'top', align: 'end',
							labels: {
								font: { family: 'JetBrains Mono, monospace', size: 10 },
								color: '#94a3b8', boxWidth: 10, boxHeight: 10,
								padding: 8, usePointStyle: true, pointStyle: 'rect',
							},
						},
						tooltip: {
							callbacks: {
								label(ctx: any) { return ` ${ctx.dataset.label}: ${formatMXN(ctx.parsed.y ?? 0)}`; },
							},
						},
					},
					scales: {
						x: {
							grid: { display: false },
							ticks: { font: { family: 'JetBrains Mono, monospace', size: 10 }, color: '#94a3b8' },
						},
						y: {
							beginAtZero: true,
							grid: { color: 'rgba(0,0,0,0.06)' },
							ticks: {
								font: { family: 'JetBrains Mono, monospace', size: 9 },
								color: '#94a3b8',
								callback(val: any) {
									const n = Number(val);
									if (n >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`;
									if (n >= 1_000) return `$${(n / 1_000).toFixed(0)}k`;
									return `$${n}`;
								},
							},
						},
					},
				},
			} as any);
		chartMes = c;
		return () => { c.destroy(); };
	});

	// Chart anual
	$effect(() => {
		if (!datosActual || !canvasAnual) return;
			const ventasActual  = MESES_CORTOS.map((_, i) => getPeriodo(datosActual,       anioActual,     i).ventas_importe);
			const comprasActual = MESES_CORTOS.map((_, i) => getPeriodo(datosActual,       anioActual,     i).compras_importe);
			const abonosActual  = MESES_CORTOS.map((_, i) => getPeriodo(datosActual,       anioActual,     i).abonos_importe);
			const ventasAnt     = MESES_CORTOS.map((_, i) => getPeriodo(datosAnioAnterior, anioActual - 1, i).ventas_importe);
			const comprasAnt    = MESES_CORTOS.map((_, i) => getPeriodo(datosAnioAnterior, anioActual - 1, i).compras_importe);
			const abonosAnt     = MESES_CORTOS.map((_, i) => getPeriodo(datosAnioAnterior, anioActual - 1, i).abonos_importe);

			const c = new Chart(canvasAnual, {
				type: 'bar',
				data: {
					labels: MESES_CORTOS,
					datasets: [
						{
							label: `Contado ${anioActual}`,
							data: ventasActual,
							backgroundColor: ventasActual.map((_, i) => i === mesActual ? 'rgba(31,146,84,0.85)' : 'rgba(31,146,84,0.22)'),
							borderColor: ventasActual.map((_, i) => i === mesActual ? 'rgba(31,146,84,1)' : 'rgba(31,146,84,0.4)'),
							borderWidth: 1, borderRadius: 2,
						},
						{
							label: `Contado ${anioActual - 1}`,
							data: ventasAnt,
							backgroundColor: ventasAnt.map((_, i) => i === mesActual ? 'rgba(31,146,84,0.35)' : 'rgba(31,146,84,0.13)'),
							borderColor: 'rgba(31,146,84,0.35)',
							borderWidth: 1, borderRadius: 2,
						},
						{
							label: `Compras ${anioActual}`,
							data: comprasActual,
							backgroundColor: comprasActual.map((_, i) => i === mesActual ? 'rgba(232,130,10,0.85)' : 'rgba(232,130,10,0.22)'),
							borderColor: comprasActual.map((_, i) => i === mesActual ? 'rgba(232,130,10,1)' : 'rgba(232,130,10,0.4)'),
							borderWidth: 1, borderRadius: 2,
						},
						{
							label: `Compras ${anioActual - 1}`,
							data: comprasAnt,
							backgroundColor: comprasAnt.map((_, i) => i === mesActual ? 'rgba(232,130,10,0.35)' : 'rgba(232,130,10,0.13)'),
							borderColor: 'rgba(232,130,10,0.35)',
							borderWidth: 1, borderRadius: 2,
						},
						{
							label: `Abonos ${anioActual}`,
							data: abonosActual,
							backgroundColor: abonosActual.map((_, i) => i === mesActual ? 'rgba(139,92,246,0.85)' : 'rgba(139,92,246,0.22)'),
							borderColor: abonosActual.map((_, i) => i === mesActual ? 'rgba(139,92,246,1)' : 'rgba(139,92,246,0.4)'),
							borderWidth: 1, borderRadius: 2,
						},
						{
							label: `Abonos ${anioActual - 1}`,
							data: abonosAnt,
							backgroundColor: abonosAnt.map((_, i) => i === mesActual ? 'rgba(139,92,246,0.35)' : 'rgba(139,92,246,0.13)'),
							borderColor: 'rgba(139,92,246,0.35)',
							borderWidth: 1, borderRadius: 2,
						},
					],
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: {
							display: true, position: 'top', align: 'end',
							labels: {
								font: { family: 'Barlow, sans-serif', size: 10 },
								color: '#94a3b8', boxWidth: 10, boxHeight: 10,
								padding: 8, usePointStyle: true, pointStyle: 'rect',
							},
						},
						tooltip: {
							callbacks: {
								label(ctx: any) { return ` ${ctx.dataset.label}: ${formatMXN(ctx.parsed.y ?? 0)}`; },
							},
						},
					},
					scales: {
						x: {
							grid: { display: false },
							ticks: { font: { family: 'JetBrains Mono, monospace', size: 10 }, color: '#94a3b8' },
						},
						y: {
							beginAtZero: true,
							grid: { color: 'rgba(0,0,0,0.06)' },
							ticks: {
								font: { family: 'JetBrains Mono, monospace', size: 9 },
								color: '#94a3b8',
								callback(val: any) {
									const n = Number(val);
									if (n >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`;
									if (n >= 1_000) return `$${(n / 1_000).toFixed(0)}k`;
									return `$${n}`;
								},
							},
						},
					},
				},
			} as any);
		chartAnual = c;
		return () => { c.destroy(); };
	});

	onDestroy(() => { chartMes?.destroy(); chartAnual?.destroy(); });

	function fmtDelta(d: number | null): string {
		if (d === null) return '—';
		return (d >= 0 ? '↑ ' : '↓ ') + Math.abs(d).toFixed(1) + '%';
	}
	function deltaClass(d: number | null): string {
		if (d === null) return 'text-slate-300';
		return d >= 0 ? 'text-green' : 'text-red-500';
	}
</script>

<div class="min-h-screen bg-bg">

	<!-- ── Header con tabs integrados ── -->
	<div class="bg-[#0f1f38] px-4 pt-5 pb-0 md:px-6">
		<div class="flex items-center gap-3 pb-4">
			<div class="w-8 h-8 flex items-center justify-center rounded bg-white/10">
				<svg class="w-4 h-4 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
					<line x1="18" y1="20" x2="18" y2="10" />
					<line x1="12" y1="20" x2="12" y2="4" />
					<line x1="6" y1="20" x2="6" y2="14" />
				</svg>
			</div>
			<div>
				<p class="text-[10px] font-mono font-semibold tracking-[0.14em] uppercase text-white/40">Análisis · {anioActual}</p>
				<h1 class="font-barlow-condensed text-[20px] font-bold text-white leading-none">Estadísticas</h1>
			</div>
			<button
				onclick={cargar}
				disabled={cargando}
				class="ml-auto w-8 h-8 flex items-center justify-center rounded bg-white/10 hover:bg-white/20 active:bg-white/30 transition-colors disabled:opacity-40"
				title="Actualizar"
			>
				<svg class="w-4 h-4 text-white {cargando ? 'animate-spin' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
					<path d="M23 4v6h-6" />
					<path d="M1 20v-6h6" />
					<path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" />
				</svg>
			</button>
		</div>

		<!-- Tabs flush al fondo, -mb-px perfora el borde inferior -->
		<div class="flex items-end gap-0 -mb-px">
			<button
				onclick={() => { tabActiva = 'detalles'; }}
				class="px-4 pb-3 pt-1 font-barlow text-[13px] font-medium border-b-2 transition-colors duration-150 {tabActiva === 'detalles' ? 'text-white border-amber' : 'text-white/45 border-transparent hover:text-white/70'}"
			>
				Detalles
			</button>
			<button
				onclick={() => { tabActiva = 'vision'; }}
				class="px-4 pb-3 pt-1 font-barlow text-[13px] font-medium border-b-2 transition-colors duration-150 {tabActiva === 'vision' ? 'text-white border-amber' : 'text-white/45 border-transparent hover:text-white/70'}"
			>
				Visión General
			</button>
		</div>
	</div>
	<!-- Separador gradiente bajo el header -->
	<div class="h-px" style="background: linear-gradient(90deg, rgba(232,130,10,0.35) 0%, rgba(226,232,240,0.8) 40%, transparent 100%)"></div>

	<div class="px-4 py-4 md:px-6 space-y-3">

		{#if sinArchivo}
			<div class="bg-surface rounded p-8 border border-slate-200 flex flex-col items-center text-center animate-fadeSlide">
				<div class="w-12 h-12 rounded flex items-center justify-center mb-4 bg-amber/10">
					<svg class="w-6 h-6 text-amber" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
						<path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
						<polyline points="14 2 14 8 20 8" />
					</svg>
				</div>
				<h2 class="font-barlow-condensed text-[20px] font-bold text-navy mb-2">Archivo no configurado</h2>
				<p class="text-[13px] text-slate-500 mb-5 max-w-xs">
					Selecciona el archivo <span class="font-mono font-semibold">Docum.DBF</span> en la configuración para ver las estadísticas.
				</p>
				<button
					onclick={onGoConfig}
					class="h-9 px-5 rounded bg-navy text-white text-[13px] font-medium font-barlow hover:opacity-90 active:opacity-80 transition-opacity"
				>
					Ir a Configuración
				</button>
			</div>

		{:else if errorMsg}
			<div class="bg-red-50 border border-red-200 rounded px-4 py-3 text-[13px] text-red-600 font-mono animate-fadeSlide">
				{errorMsg}
			</div>

		{:else if cargando}
			<!-- Skeleton con forma del nuevo layout -->
			<div class="flex gap-2 pt-1">
				<div class="h-5 w-20 rounded-full animate-shimmer" style="background: linear-gradient(90deg, #c8d4e8 25%, #d8e2f0 50%, #c8d4e8 75%); background-size: 400% 100%"></div>
				<div class="h-5 w-28 rounded-full animate-shimmer" style="background: linear-gradient(90deg, #c8d4e8 25%, #d8e2f0 50%, #c8d4e8 75%); background-size: 400% 100%"></div>
			</div>
			<div class="h-12 w-48 rounded animate-shimmer" style="background: linear-gradient(90deg, #d8e2f0 25%, #e8eef6 50%, #d8e2f0 75%); background-size: 400% 100%"></div>
			<div class="grid grid-cols-3 gap-2">
				{#each [0,1,2] as _}
					<div class="h-20 rounded border border-slate-100 animate-shimmer" style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%"></div>
				{/each}
			</div>
			<div class="h-16 rounded border border-slate-100 animate-shimmer" style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%"></div>
			<div class="h-52 rounded border border-slate-100 animate-shimmer" style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%"></div>
			<div class="h-44 rounded border border-slate-100 animate-shimmer" style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%"></div>

		{:else if datosActual}

			{#key tabActiva}
			<div class="animate-fadeSlide">

				{#if tabActiva === 'detalles'}

					<!-- ── Navegador de mes ── -->
					<div class="flex items-center gap-3 -mx-4 md:-mx-6 px-4 md:px-6 py-3 border-b border-slate-200/70 bg-surface/60 mb-4">
						<button
							onclick={() => navMes(-1)}
							disabled={mesSeleccionado === 0}
							aria-label="Mes anterior"
							class="w-8 h-8 flex items-center justify-center rounded-lg text-navy/50 hover:text-navy hover:bg-navy/8 active:bg-navy/15 transition-colors disabled:opacity-25 disabled:pointer-events-none"
						>
							<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<polyline points="15 18 9 12 15 6" />
							</svg>
						</button>

						<div class="flex-1 text-center"
							style="transition: opacity 110ms, transform 110ms; opacity: {mesCambiando ? 0 : 1}; transform: translateY({mesCambiando ? '4px' : '0'})">
							<h2 class="font-barlow-condensed text-[30px] font-bold text-navy leading-none">
								{MESES_COMPLETOS[mesSeleccionado]}
							</h2>
							<p class="text-[9px] font-mono font-semibold tracking-[0.16em] uppercase text-slate-400 mt-0.5">
								{anioActual} · vs {MESES_CORTOS[mesSeleccionado]} {anioActual - 1}
							</p>
						</div>

						<button
							onclick={() => navMes(1)}
							disabled={mesSeleccionado >= mesActual}
							aria-label="Mes siguiente"
							class="w-8 h-8 flex items-center justify-center rounded-lg text-navy/50 hover:text-navy hover:bg-navy/8 active:bg-navy/15 transition-colors disabled:opacity-25 disabled:pointer-events-none"
						>
							<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<polyline points="9 18 15 12 9 6" />
							</svg>
						</button>
					</div>

					<!-- ── KPI cards: 3 columnas ── -->
					<div class="grid grid-cols-3 gap-2 animate-fadeSlide" style="animation-delay: 30ms">

						<!-- Ventas -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-3 py-3 overflow-hidden" style="border-left-color: #1f9254">
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-1.5">Ventas</p>
							<div class="flex items-start justify-between gap-1 mb-1.5">
								<span class="font-barlow-condensed text-[20px] font-bold text-navy leading-none">{formatMXN(mesData.ventas_importe)}</span>
								{#if deltaVentas !== null}
									<span class="shrink-0 text-[10px] font-mono font-bold px-1.5 py-0.5 rounded-full leading-none mt-0.5 {deltaVentas >= 0 ? 'bg-green/10 text-green' : 'bg-red-50 text-red-500'}">
										{deltaVentas >= 0 ? '↑' : '↓'} {Math.abs(deltaVentas).toFixed(1)}%
									</span>
								{:else}
									<span class="text-[10px] font-mono text-slate-300">—</span>
								{/if}
							</div>
							<p class="text-[9px] font-mono text-slate-400 truncate">vs {formatMXN(mesPrevData.ventas_importe)}</p>
						</div>

						<!-- Compras -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-3 py-3 overflow-hidden" style="border-left-color: #e8820a">
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-1.5">Compras</p>
							<div class="flex items-start justify-between gap-1 mb-1.5">
								<span class="font-barlow-condensed text-[20px] font-bold text-navy leading-none">{formatMXN(mesData.compras_importe)}</span>
								{#if deltaCompras !== null}
									<span class="shrink-0 text-[10px] font-mono font-bold px-1.5 py-0.5 rounded-full leading-none mt-0.5 {deltaCompras >= 0 ? 'bg-amber/10 text-amber' : 'bg-green/10 text-green'}">
										{deltaCompras >= 0 ? '↑' : '↓'} {Math.abs(deltaCompras).toFixed(1)}%
									</span>
								{:else}
									<span class="text-[10px] font-mono text-slate-300">—</span>
								{/if}
							</div>
							<p class="text-[9px] font-mono text-slate-400 truncate">vs {formatMXN(mesPrevData.compras_importe)}</p>
						</div>

						<!-- Abonos -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-3 py-3 overflow-hidden" style="border-left-color: #8b5cf6">
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-1.5">Abonos CXC</p>
							<div class="flex items-start justify-between gap-1 mb-1.5">
								<span class="font-barlow-condensed text-[20px] font-bold text-navy leading-none">{formatMXN(mesData.abonos_importe)}</span>
								{#if deltaAbonos !== null}
									<span class="shrink-0 text-[10px] font-mono font-bold px-1.5 py-0.5 rounded-full leading-none mt-0.5 {deltaAbonos >= 0 ? 'bg-violet-50 text-violet-600' : 'bg-red-50 text-red-500'}">
										{deltaAbonos >= 0 ? '↑' : '↓'} {Math.abs(deltaAbonos).toFixed(1)}%
									</span>
								{:else}
									<span class="text-[10px] font-mono text-slate-300">—</span>
								{/if}
							</div>
							<p class="text-[9px] font-mono text-slate-400 truncate">vs {formatMXN(mesPrevData.abonos_importe)}</p>
						</div>
					</div>

					<!-- Balance card ancho completo -->
					<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-4 py-3 flex items-center justify-between animate-fadeSlide overflow-hidden"
						style="animation-delay: 60ms; border-left-color: {balanceActual >= 0 ? '#1f9254' : '#ef4444'}">
						<div>
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-0.5">Balance General</p>
							<p class="text-[9px] font-mono text-slate-400">Ventas + Abonos − Compras</p>
						</div>
						<div class="text-right">
							<span class="font-barlow-condensed text-[34px] font-bold leading-none {balanceActual >= 0 ? 'text-green' : 'text-red-500'}">
								{formatMXN(balanceActual)}
							</span>
							{#if deltaBalance !== null}
								<p class="text-[9px] font-mono {deltaBalance >= 0 ? 'text-green' : 'text-red-400'} mt-0.5">
									{deltaBalance >= 0 ? '↑' : '↓'} {Math.abs(deltaBalance).toFixed(1)}% vs {anioActual - 1}
								</p>
							{/if}
						</div>
					</div>

					<!-- Gráfico barras mensual -->
					<div class="bg-surface rounded p-4 border border-slate-200 animate-fadeSlide" style="animation-delay: 100ms">
						<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-3">
							{MESES_COMPLETOS[mesSeleccionado].toUpperCase()} — {anioActual} vs {anioActual - 1}
						</p>
						<div class="relative h-48 sm:h-56">
							<canvas bind:this={canvasMes}></canvas>
						</div>
					</div>

					<!-- Tabla desglose 3 columnas -->
					<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 140ms">

						<div class="grid grid-cols-3 bg-bg border-b border-slate-100">
							<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2 border-r border-slate-100">
								<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-green pr-3">Ventas</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual}</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual - 1}</p>
							</div>
							<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2 border-r border-slate-100">
								<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-amber pr-3">Compras</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual}</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual - 1}</p>
							</div>
							<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2">
								<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-blue-600 pr-3">Cobranza</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual}</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual - 1}</p>
							</div>
						</div>

						<div class="grid grid-cols-3 divide-x divide-slate-100">
							<!-- Col 1: Ventas -->
							<div class="flex flex-col">
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[12px] text-slate-600 pr-3">Facturas</span>
									<span class="font-mono text-[12px] text-green text-right">{formatMXN(mesData.facturas_importe)}</span>
									<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevData.facturas_importe)}</span>
								</div>
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[12px] text-slate-600 pr-3">Remisiones</span>
									<span class="font-mono text-[12px] text-green text-right">{formatMXN(mesData.remisiones_importe)}</span>
									<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevData.remisiones_importe)}</span>
								</div>
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[12px] text-slate-600 pr-3">Notas</span>
									<span class="font-mono text-[12px] text-green text-right">{formatMXN(mesData.notas_importe)}</span>
									<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevData.notas_importe)}</span>
								</div>
								<div class="mt-auto grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-t border-slate-200 bg-green/[0.04] items-center">
									<span class="text-[11px] font-mono font-bold text-slate-500 uppercase tracking-wider pr-3">Total</span>
									<span class="font-mono text-[13px] font-bold text-green text-right">{formatMXN(mesData.ventas_importe)}</span>
									<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevData.ventas_importe)}</span>
								</div>
							</div>
							<!-- Col 2: Compras -->
							<div class="flex flex-col">
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[12px] text-slate-600 pr-3">Compras</span>
									<span class="font-mono text-[12px] text-amber text-right">{formatMXN(mesData.compras_importe)}</span>
									<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevData.compras_importe)}</span>
								</div>
								<div class="mt-auto grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-t border-slate-200 bg-amber/[0.04] items-center">
									<span class="text-[11px] font-mono font-bold text-slate-500 uppercase tracking-wider pr-3">Total</span>
									<span class="font-mono text-[13px] font-bold text-amber text-right">{formatMXN(mesData.compras_importe)}</span>
									<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevData.compras_importe)}</span>
								</div>
							</div>
							<!-- Col 3: Cobranza -->
							<div class="flex flex-col">
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[12px] text-slate-600 pr-3">Abonos CXC</span>
									<span class="font-mono text-[12px] text-violet-600 text-right">{formatMXN(mesData.abonos_importe)}</span>
									<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevData.abonos_importe)}</span>
								</div>
								<div class="mt-auto grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-t border-slate-200 bg-violet-500/[0.04] items-center">
									<span class="text-[11px] font-mono font-bold text-slate-500 uppercase tracking-wider pr-3">Total</span>
									<span class="font-mono text-[13px] font-bold text-violet-600 text-right">{formatMXN(mesData.abonos_importe)}</span>
									<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevData.abonos_importe)}</span>
								</div>
							</div>
						</div>

					</div>

				{:else}

					<!-- ── VISIÓN GENERAL ── -->

					<!-- KPI anuales: grid 2x2 -->
					<div class="grid grid-cols-2 gap-2 animate-fadeSlide" style="animation-delay: 20ms">

						<!-- Total Ventas -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-3 py-3 overflow-hidden" style="border-left-color: #1f9254">
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-1.5">Total Ventas {anioActual}</p>
							<span class="font-barlow-condensed text-[22px] font-bold text-navy leading-none block mb-1">
								{formatMXN(datosActual.total_ventas)}
							</span>
							{#if datosAnioAnterior}
								{@const dv = deltaPct(datosActual.total_ventas, datosAnioAnterior.total_ventas)}
								<p class="text-[9px] font-mono text-slate-400">
									vs {formatMXN(datosAnioAnterior.total_ventas)}
									{#if dv !== null}
										<span class="{dv >= 0 ? 'text-green' : 'text-red-400'} font-semibold ml-1">{dv >= 0 ? '↑' : '↓'}{Math.abs(dv).toFixed(1)}%</span>
									{/if}
								</p>
							{/if}
						</div>

						<!-- Total Compras -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-3 py-3 overflow-hidden" style="border-left-color: #e8820a">
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-1.5">Total Compras {anioActual}</p>
							<span class="font-barlow-condensed text-[22px] font-bold text-navy leading-none block mb-1">
								{formatMXN(datosActual.total_compras)}
							</span>
							{#if datosAnioAnterior}
								{@const dc = deltaPct(datosActual.total_compras, datosAnioAnterior.total_compras)}
								<p class="text-[9px] font-mono text-slate-400">
									vs {formatMXN(datosAnioAnterior.total_compras)}
									{#if dc !== null}
										<span class="{dc >= 0 ? 'text-amber' : 'text-green'} font-semibold ml-1">{dc >= 0 ? '↑' : '↓'}{Math.abs(dc).toFixed(1)}%</span>
									{/if}
								</p>
							{/if}
						</div>

						<!-- Total Abonos -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-3 py-3 overflow-hidden" style="border-left-color: #8b5cf6">
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-1.5">Total Abonos {anioActual}</p>
							<span class="font-barlow-condensed text-[22px] font-bold text-navy leading-none block mb-1">
								{formatMXN(datosActual.total_abonos)}
							</span>
							{#if datosAnioAnterior}
								{@const da = deltaPct(datosActual.total_abonos, datosAnioAnterior.total_abonos)}
								<p class="text-[9px] font-mono text-slate-400">
									vs {formatMXN(datosAnioAnterior.total_abonos)}
									{#if da !== null}
										<span class="{da >= 0 ? 'text-violet-500' : 'text-red-400'} font-semibold ml-1">{da >= 0 ? '↑' : '↓'}{Math.abs(da).toFixed(1)}%</span>
									{/if}
								</p>
							{/if}
						</div>

						<!-- Balance Anual -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-3 py-3 overflow-hidden" style="border-left-color: {balAnual >= 0 ? '#1f9254' : '#ef4444'}">
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-1.5">Balance Anual {anioActual}</p>
							<span class="font-barlow-condensed text-[22px] font-bold leading-none block mb-1 {balAnual >= 0 ? 'text-green' : 'text-red-500'}">
								{formatMXN(balAnual)}
							</span>
							{#if deltaBalAnual !== null}
								<p class="text-[9px] font-mono {deltaBalAnual >= 0 ? 'text-green' : 'text-red-400'}">
									{deltaBalAnual >= 0 ? '↑' : '↓'} {Math.abs(deltaBalAnual).toFixed(1)}% vs {anioActual - 1}
								</p>
							{/if}
						</div>

					</div>

					<!-- Gráfico tendencia anual -->
					<div class="bg-surface rounded p-4 border border-slate-200 animate-fadeSlide" style="animation-delay: 60ms">
						<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-3">
							TENDENCIA ANUAL — {anioActual} vs {anioActual - 1}
						</p>
						<div class="relative h-48 sm:h-60">
							<canvas bind:this={canvasAnual}></canvas>
						</div>
					</div>

					<!-- Tabla 12 meses -->
					<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 100ms">

						<!-- Cabecera -->
						<div class="grid grid-cols-[1fr_1fr_1fr_1fr] bg-bg border-b border-slate-200 px-3 py-2 gap-2">
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-slate-400">Mes</span>
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-green text-right">Ventas</span>
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-amber text-right">Compras</span>
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-violet-500 text-right">Abonos</span>
						</div>

						<!-- Filas mensuales -->
						{#each filasMeses as fila}
							{@const esActual = fila.idx === mesActual}
							<div
								class="grid grid-cols-[1fr_1fr_1fr_1fr] px-3 py-2 gap-2 border-b border-slate-50 items-center
									{esActual ? 'bg-amber/[0.035] border-l-[3px] border-amber' : 'border-l-[3px] border-transparent'}"
							>
								<span class="text-[12px] {esActual ? 'text-navy font-semibold' : 'text-slate-600'} font-barlow truncate">
									{fila.nombre}
								</span>
								<div class="text-right">
									<span class="font-mono text-[11px] text-navy block">{formatMXN(fila.act.ventas_importe)}</span>
									<span class="font-mono text-[9px] {deltaClass(fila.deltaV)}">{fmtDelta(fila.deltaV)}</span>
								</div>
								<div class="text-right">
									<span class="font-mono text-[11px] text-navy block">{formatMXN(fila.act.compras_importe)}</span>
									<span class="font-mono text-[9px] {deltaClass(fila.deltaC)}">{fmtDelta(fila.deltaC)}</span>
								</div>
								<div class="text-right">
									<span class="font-mono text-[11px] text-navy block">{formatMXN(fila.act.abonos_importe)}</span>
									<span class="font-mono text-[9px] {deltaClass(fila.deltaA)}">{fmtDelta(fila.deltaA)}</span>
								</div>
							</div>
						{/each}

						<!-- Fila de totales -->
						<div class="grid grid-cols-[1fr_1fr_1fr_1fr] px-3 py-2.5 gap-2 items-center bg-navy/[0.05] border-t-2 border-navy/20">
							<span class="text-[10px] font-mono font-bold text-slate-500 uppercase tracking-wider">Total</span>
							<span class="font-mono text-[12px] font-bold text-green text-right">{formatMXN(datosActual.total_ventas)}</span>
							<span class="font-mono text-[12px] font-bold text-amber text-right">{formatMXN(datosActual.total_compras)}</span>
							<span class="font-mono text-[12px] font-bold text-violet-600 text-right">{formatMXN(datosActual.total_abonos)}</span>
						</div>

					</div>

				{/if}

			</div>
			{/key}

		{/if}

	</div>
</div>
