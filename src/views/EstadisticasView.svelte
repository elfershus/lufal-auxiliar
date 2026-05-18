<script lang="ts">
	import { onDestroy } from 'svelte';
	import {
		Chart, BarController, BarElement,
		CategoryScale, LinearScale, Tooltip, Legend
	} from 'chart.js';
	import { getEstadisticasDocum } from '../lib/dbf.js';
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

	let cargando = $state(true);
	let errorMsg = $state('');
	let sinArchivo = $state(false);
	let datosActual = $state<EstadisticasResult | null>(null);
	let datosAnioAnterior = $state<EstadisticasResult | null>(null);
	let canvasMes = $state<HTMLCanvasElement | null>(null);
	let canvasAnual = $state<HTMLCanvasElement | null>(null);
	let chartMes: Chart | null = null;
	let chartAnual: Chart | null = null;

	function getPeriodo(datos: EstadisticasResult | null, anio: number, mesIdx: number): PeriodoStat {
		const key = `${anio}-${String(mesIdx + 1).padStart(2, '0')}`;
		return datos?.periodos.find(p => p.periodo === key) ?? {
			periodo: key,
			ventas_importe: 0, compras_importe: 0,
			ventas_count: 0, compras_count: 0,
			facturas_importe: 0, facturas_count: 0,
			remisiones_importe: 0, remisiones_count: 0,
			notas_importe: 0, notas_count: 0,
		};
	}

	function deltaPct(actual: number, anterior: number): number | null {
		if (anterior === 0) return null;
		return ((actual - anterior) / anterior) * 100;
	}

	async function cargar() {
		cargando = true;
		errorMsg = '';
		sinArchivo = false;
		datosActual = null;
		datosAnioAnterior = null;

		try {
			datosActual = await getEstadisticasDocum(`${anioActual}-01-01`, `${anioActual}-12-31`);
			try {
				datosAnioAnterior = await getEstadisticasDocum(`${anioActual - 1}-01-01`, `${anioActual - 1}-12-31`);
			} catch { /* año anterior mostrará ceros */ }
		} catch (e) {
			const msg = e instanceof Error ? e.message : String(e);
			if (msg.includes('no configurado')) sinArchivo = true;
			else errorMsg = msg;
		}
		cargando = false;
	}

	$effect(() => { cargar(); });

	const mesActualData = $derived(getPeriodo(datosActual,       anioActual,     mesActual));
	const mesPrevioData = $derived(getPeriodo(datosAnioAnterior, anioActual - 1, mesActual));
	const mesPrevioNombre = $derived(`${MESES_CORTOS[mesActual]} ${anioActual - 1}`);

	const deltaVentas  = $derived(deltaPct(mesActualData.ventas_importe,  mesPrevioData.ventas_importe));
	const deltaCompras = $derived(deltaPct(mesActualData.compras_importe, mesPrevioData.compras_importe));

	const balanceActual = $derived(mesActualData.ventas_importe - mesActualData.compras_importe);
	const balancePrevio = $derived(mesPrevioData.ventas_importe - mesPrevioData.compras_importe);
	const balanceMejoro = $derived(balanceActual > balancePrevio);
	const diffBalance   = $derived(Math.abs(balanceActual - balancePrevio));

	// Gráfico de 4 barras: Ventas/Compras año actual vs año anterior
	$effect(() => {
		if (datosActual && canvasMes) {
			chartMes?.destroy();
			chartMes = new Chart(canvasMes, {
				type: 'bar',
				data: {
					labels: ['Ventas', 'Compras'],
					datasets: [
						{
							label: String(anioActual),
							data: [mesActualData.ventas_importe, mesActualData.compras_importe],
							backgroundColor: ['rgba(31,146,84,0.85)', 'rgba(232,130,10,0.85)'],
							borderColor: ['rgba(31,146,84,1)', 'rgba(232,130,10,1)'],
							borderWidth: 1,
							borderRadius: 4,
						},
						{
							label: String(anioActual - 1),
							data: [mesPrevioData.ventas_importe, mesPrevioData.compras_importe],
							backgroundColor: ['rgba(31,146,84,0.25)', 'rgba(232,130,10,0.25)'],
							borderColor: ['rgba(31,146,84,0.45)', 'rgba(232,130,10,0.45)'],
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
							display: true,
							position: 'top',
							align: 'end',
							labels: {
								font: { family: 'JetBrains Mono, monospace', size: 10 },
								color: '#94a3b8',
								boxWidth: 10,
								boxHeight: 10,
								padding: 8,
								usePointStyle: true,
								pointStyle: 'rect',
							},
						},
						tooltip: {
							callbacks: {
								label(ctx) {
									return ` ${ctx.dataset.label}: ${formatMXN(ctx.parsed.y ?? 0)}`;
								},
							},
						},
					},
					scales: {
						x: {
							grid: { display: false },
							ticks: {
								font: { family: 'JetBrains Mono, monospace', size: 10 },
								color: '#94a3b8',
							},
						},
						y: {
							beginAtZero: true,
							grid: { color: 'rgba(0,0,0,0.06)' },
							ticks: {
								font: { family: 'JetBrains Mono, monospace', size: 9 },
								color: '#94a3b8',
								callback(val) {
									const n = Number(val);
									if (n >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`;
									if (n >= 1_000) return `$${(n / 1_000).toFixed(0)}k`;
									return `$${n}`;
								},
							},
						},
					},
				},
			});
		}
	});

	// Gráfico de tendencia anual: barras año actual + líneas año anterior
	$effect(() => {
		if (datosActual && canvasAnual) {
			chartAnual?.destroy();

			const ventasActual  = MESES_CORTOS.map((_, i) => getPeriodo(datosActual,       anioActual,     i).ventas_importe);
			const comprasActual = MESES_CORTOS.map((_, i) => getPeriodo(datosActual,       anioActual,     i).compras_importe);
			const ventasAnt     = MESES_CORTOS.map((_, i) => getPeriodo(datosAnioAnterior, anioActual - 1, i).ventas_importe);
			const comprasAnt    = MESES_CORTOS.map((_, i) => getPeriodo(datosAnioAnterior, anioActual - 1, i).compras_importe);

			chartAnual = new Chart(canvasAnual, {
				type: 'bar',
				data: {
					labels: MESES_CORTOS,
					datasets: [
						{
							label: `Ventas ${anioActual}`,
							data: ventasActual,
							backgroundColor: ventasActual.map((_, i) =>
								i === mesActual ? 'rgba(31,146,84,0.85)' : 'rgba(31,146,84,0.22)'
							),
							borderColor: ventasActual.map((_, i) =>
								i === mesActual ? 'rgba(31,146,84,1)' : 'rgba(31,146,84,0.4)'
							),
							borderWidth: 1,
							borderRadius: 2,
						},
						{
							label: `Ventas ${anioActual - 1}`,
							data: ventasAnt,
							backgroundColor: ventasAnt.map((_, i) =>
								i === mesActual ? 'rgba(31,146,84,0.35)' : 'rgba(31,146,84,0.13)'
							),
							borderColor: 'rgba(31,146,84,0.35)',
							borderWidth: 1,
							borderRadius: 2,
						},
						{
							label: `Compras ${anioActual}`,
							data: comprasActual,
							backgroundColor: comprasActual.map((_, i) =>
								i === mesActual ? 'rgba(232,130,10,0.85)' : 'rgba(232,130,10,0.22)'
							),
							borderColor: comprasActual.map((_, i) =>
								i === mesActual ? 'rgba(232,130,10,1)' : 'rgba(232,130,10,0.4)'
							),
							borderWidth: 1,
							borderRadius: 2,
						},
						{
							label: `Compras ${anioActual - 1}`,
							data: comprasAnt,
							backgroundColor: comprasAnt.map((_, i) =>
								i === mesActual ? 'rgba(232,130,10,0.35)' : 'rgba(232,130,10,0.13)'
							),
							borderColor: 'rgba(232,130,10,0.35)',
							borderWidth: 1,
							borderRadius: 2,
						},
					],
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: {
							display: true,
							position: 'top',
							align: 'end',
							labels: {
								font: { family: 'Barlow, sans-serif', size: 10 },
								color: '#94a3b8',
								boxWidth: 10,
								boxHeight: 10,
								padding: 8,
								usePointStyle: true,
								pointStyle: 'rect',
							},
						},
						tooltip: {
							callbacks: {
								label(ctx) { return ` ${ctx.dataset.label}: ${formatMXN(ctx.parsed.y ?? 0)}`; },
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
								callback(val) {
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
		}
	});

	onDestroy(() => { chartMes?.destroy(); chartAnual?.destroy(); });
</script>

<div class="min-h-screen bg-bg">
	<!-- Header -->
	<div class="bg-[#0f1f38] px-4 pt-5 pb-4 md:px-6 flex items-center gap-3">
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
	</div>

	<div class="px-4 py-4 md:px-6 max-w-4xl space-y-3">

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
			<div class="h-7 w-44 rounded mb-1 animate-shimmer"
				style="background: linear-gradient(90deg, #c8d4e8 25%, #d8e2f0 50%, #c8d4e8 75%); background-size: 400% 100%">
			</div>
			<div class="h-3 w-36 rounded mb-4 animate-shimmer"
				style="background: linear-gradient(90deg, #d8e2f0 25%, #e8eef6 50%, #d8e2f0 75%); background-size: 400% 100%">
			</div>
			<div class="h-56 rounded border border-slate-100 mb-3 animate-shimmer"
				style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
			</div>
			<div class="h-12 rounded border border-slate-100 mb-3 animate-shimmer"
				style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
			</div>
			<div class="h-28 rounded border border-slate-100 mb-3 animate-shimmer"
				style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
			</div>
			<div class="h-44 rounded border border-slate-100 animate-shimmer"
				style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
			</div>

		{:else if datosActual}

			<!-- Título del mes -->
			<div class="animate-fadeSlide pt-1">
				<div class="flex items-baseline gap-2">
					<h2 class="font-barlow-condensed text-[30px] font-bold text-navy leading-none">{MESES_COMPLETOS[mesActual]}</h2>
					<span class="font-mono text-[13px] font-bold text-slate-400">{anioActual}</span>
				</div>
				<p class="text-[9px] font-mono font-semibold tracking-[0.16em] uppercase text-slate-400 mt-0.5">
					vs {mesPrevioNombre} — año vs año
				</p>
			</div>

			<!-- Gráfico de 4 barras: comparativa del mes actual vs mismo mes año anterior -->
			<div class="bg-surface rounded p-4 border border-slate-200 animate-fadeSlide" style="animation-delay: 40ms">
				<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-3">
					{MESES_COMPLETOS[mesActual].toUpperCase()} — {anioActual} vs {anioActual - 1}
				</p>
				<div class="relative h-48 sm:h-56">
					<canvas bind:this={canvasMes}></canvas>
				</div>
			</div>

			<!-- Desglose unificado + Balance neto -->
			<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 80ms">

				<!-- Cabeceras de columna con sub-años alineados -->
				<div class="grid grid-cols-2 bg-bg border-b border-slate-100">
					<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2 border-r border-slate-100">
						<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-green pr-3">Ventas</p>
						<p class="text-right text-[9px] font-mono text-slate-400">{anioActual}</p>
						<p class="text-right text-[9px] font-mono text-slate-400">{anioActual - 1}</p>
					</div>
					<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2">
						<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-amber pr-3">Compras</p>
						<p class="text-right text-[9px] font-mono text-slate-400">{anioActual}</p>
						<p class="text-right text-[9px] font-mono text-slate-400">{anioActual - 1}</p>
					</div>
				</div>

				<!-- Dos columnas de detalle -->
				<div class="grid grid-cols-2 divide-x divide-slate-100">

					<!-- Columna izquierda: ventas -->
					<div class="flex flex-col">
						<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
							<span class="text-[12px] text-slate-600 pr-3">Facturas</span>
							<span class="font-mono text-[12px] text-green text-right">{formatMXN(mesActualData.facturas_importe)}</span>
							<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevioData.facturas_importe)}</span>
						</div>
						<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
							<span class="text-[12px] text-slate-600 pr-3">Remisiones</span>
							<span class="font-mono text-[12px] text-green text-right">{formatMXN(mesActualData.remisiones_importe)}</span>
							<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevioData.remisiones_importe)}</span>
						</div>
						<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
							<span class="text-[12px] text-slate-600 pr-3">Notas</span>
							<span class="font-mono text-[12px] text-green text-right">{formatMXN(mesActualData.notas_importe)}</span>
							<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevioData.notas_importe)}</span>
						</div>
						<div class="mt-auto grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-t border-slate-200 bg-green/[0.04] items-center">
							<span class="text-[11px] font-mono font-bold text-slate-500 uppercase tracking-wider pr-3">Total</span>
							<span class="font-mono text-[13px] font-bold text-green text-right">{formatMXN(mesActualData.ventas_importe)}</span>
							<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevioData.ventas_importe)}</span>
						</div>
					</div>

					<!-- Columna derecha: compras -->
					<div class="flex flex-col">
						<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-b border-slate-50 items-center">
							<span class="text-[12px] text-slate-600 pr-3">Compras</span>
							<span class="font-mono text-[12px] text-amber text-right">{formatMXN(mesActualData.compras_importe)}</span>
							<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevioData.compras_importe)}</span>
						</div>
						<div class="mt-auto grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 sm:px-4 py-2.5 border-t border-slate-200 bg-amber/[0.04] items-center">
							<span class="text-[11px] font-mono font-bold text-slate-500 uppercase tracking-wider pr-3">Total</span>
							<span class="font-mono text-[13px] font-bold text-amber text-right">{formatMXN(mesActualData.compras_importe)}</span>
							<span class="font-mono text-[12px] text-slate-400 text-right">{formatMXN(mesPrevioData.compras_importe)}</span>
						</div>
					</div>

				</div>

				<!-- Franja inferior: diferencia ventas − compras -->
				<div class="border-t-2 border-slate-200 px-4 py-2.5 flex items-center justify-between bg-bg/40">
					<span class="text-[11px] font-mono font-bold text-slate-500 uppercase tracking-wider">Diferencia</span>
					<span class="font-mono text-[18px] font-bold {balanceActual >= 0 ? 'text-green' : 'text-red-500'}">
						{formatMXN(balanceActual)}
					</span>
				</div>

			</div>

			<!-- Gráfico tendencia anual -->
			<div class="bg-surface rounded p-4 border border-slate-200 animate-fadeSlide" style="animation-delay: 140ms">
				<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-3">
					TENDENCIA ANUAL — {anioActual} vs {anioActual - 1}
				</p>
				<div class="relative h-40 sm:h-52">
					<canvas bind:this={canvasAnual}></canvas>
				</div>
			</div>

		{/if}

	</div>
</div>
