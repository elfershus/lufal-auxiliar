<script lang="ts">
	import { onDestroy } from 'svelte';
	import {
		Chart, BarController, BarElement,
		CategoryScale, LinearScale, Tooltip, Legend
	} from 'chart.js';
	import { getEstadisticasDosAnios, getEstadisticasInventarioDetalle, getEstadisticasCxcMensual } from '../lib/dbf.js';
	import { appConfig } from '../lib/config.svelte.js';
	import { formatMXN } from '../lib/utils.js';
	import type { EstadisticasResult, PeriodoStat, InventarioAnioResult, CxcMensualAnioResult, InventarioMesStat, CxcMensualMesStat } from '../lib/types.js';

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

	let cargando          = $state(true);
	let errorMsg          = $state('');
	let sinArchivo        = $state(false);
	let datosActual       = $state<EstadisticasResult | null>(null);
	let datosAnioAnterior = $state<EstadisticasResult | null>(null);
	let datosInventario   = $state<InventarioAnioResult | null>(null);
	let datosCxc          = $state<CxcMensualAnioResult | null>(null);
	let canvasMes      = $state<HTMLCanvasElement | null>(null);
	let canvasAnual    = $state<HTMLCanvasElement | null>(null);
	let chartMes: Chart | null = null;
	let chartAnual: Chart | null = null;

	// Tab y navegación de mes
	let tabActiva       = $state<'detalles' | 'vision'>('detalles');
	let mesSeleccionado = $state(mesActual);

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
			devoluciones_importe: 0, devoluciones_count: 0,
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
		datosInventario = null;
		datosCxc = null;

		const numalm = appConfig.numalm || undefined;
		const [resBase, resInv, resCxc] = await Promise.allSettled([
			getEstadisticasDosAnios(anioActual, numalm),
			getEstadisticasInventarioDetalle(numalm),
			getEstadisticasCxcMensual(numalm),
		]);

		if (resBase.status === 'fulfilled') {
			datosActual = resBase.value.actual;
			datosAnioAnterior = resBase.value.anterior;
		} else {
			const msg = resBase.reason instanceof Error ? resBase.reason.message : String(resBase.reason);
			if (msg.includes('no configurado')) sinArchivo = true;
			else errorMsg = msg;
		}
		if (resInv.status === 'fulfilled') datosInventario = resInv.value;
		if (resCxc.status === 'fulfilled') datosCxc = resCxc.value;

		cargando = false;
	}

	$effect(() => { cargar(); });

	// Derived para tab Detalles (reactive a mesSeleccionado)
	const mesData     = $derived(getPeriodo(datosActual,       anioActual,     mesSeleccionado));
	const mesPrevData = $derived(getPeriodo(datosAnioAnterior, anioActual - 1, mesSeleccionado));
	const gastosActual  = $derived(mesData.compras_importe + mesData.devoluciones_importe);
	const gastosPrevio  = $derived(mesPrevData.compras_importe + mesPrevData.devoluciones_importe);
	const balanceActual = $derived(mesData.ventas_importe + mesData.abonos_importe - gastosActual);
	const balancePrevio = $derived(mesPrevData.ventas_importe + mesPrevData.abonos_importe - gastosPrevio);
	const deltaBalance  = $derived(deltaPct(balanceActual, balancePrevio));

	// Derived para tab Visión General
	const filasMeses = $derived(
		MESES_CORTOS.map((_, i) => {
			const act = getPeriodo(datosActual,       anioActual,     i);
			const ant = getPeriodo(datosAnioAnterior, anioActual - 1, i);
			const esFuturo = i > mesActual;
			const mesKey = `${anioActual}-${String(i + 1).padStart(2, '0')}`;
			const invMes = datosInventario?.meses.find(m => m.mes === mesKey);
			const cxcMes = datosCxc?.meses.find(m => m.mes === mesKey);
			return {
				idx: i,
				nombre: MESES_COMPLETOS[i],
				act, ant,
				deltaV: esFuturo ? null : deltaPct(act.ventas_importe,  ant.ventas_importe),
				deltaC: esFuturo ? null : deltaPct(act.compras_importe, ant.compras_importe),
				deltaA: esFuturo ? null : deltaPct(act.abonos_importe,  ant.abonos_importe),
				difInv: invMes != null ? invMes.entradas - invMes.salidas : null,
				difCxc: cxcMes != null ? cxcMes.cargos - cxcMes.abonos : null,
			};
		})
	);

	// Totales anuales para Visión General (ya existentes en el tipo)
	const balAnual      = $derived(datosActual ? datosActual.total_ventas + datosActual.total_abonos - datosActual.total_compras : 0);
	const balAnualAnt   = $derived(datosAnioAnterior ? datosAnioAnterior.total_ventas + datosAnioAnterior.total_abonos - datosAnioAnterior.total_compras : 0);
	const deltaBalAnual    = $derived(deltaPct(balAnual, balAnualAnt));
	const deltaVentasAnual = $derived(deltaPct(datosActual?.total_ventas ?? 0, datosAnioAnterior?.total_ventas ?? 0));
	const deltaComprasAnual = $derived(deltaPct(datosActual?.total_compras ?? 0, datosAnioAnterior?.total_compras ?? 0));
	const deltaAbonosAnual  = $derived(deltaPct(datosActual?.total_abonos ?? 0, datosAnioAnterior?.total_abonos ?? 0));
	const totalInvAnual = $derived(datosInventario?.meses.reduce((s, m) => s + m.entradas - m.salidas, 0) ?? null);
	const totalCxcAnual = $derived(datosCxc?.meses.reduce((s, m) => s + m.cargos - m.abonos, 0) ?? null);

	const INV_EMPTY: InventarioMesStat = { mes: '', saldo_inicial: 0, entradas: 0, salidas: 0, saldo_final: 0 };
	const CXC_EMPTY: CxcMensualMesStat = { mes: '', saldo_inicial: 0, cargos: 0, abonos: 0, saldo_final: 0 };

	const invMesData = $derived(
		datosInventario?.meses.find(m => m.mes === `${anioActual}-${String(mesSeleccionado + 1).padStart(2, '0')}`)
		?? INV_EMPTY
	);
	const cxcMesData = $derived(
		datosCxc?.meses.find(m => m.mes === `${anioActual}-${String(mesSeleccionado + 1).padStart(2, '0')}`)
		?? CXC_EMPTY
	);
	const difInv = $derived(invMesData.entradas - invMesData.salidas);
	const difCxc = $derived(cxcMesData.cargos - cxcMesData.abonos);

	// Chart mensual: redibujar al cambiar mesSeleccionado
	$effect(() => {
		if (!datosActual || !canvasMes) return;
		void mesSeleccionado;
			const c = new Chart(canvasMes, {
				type: 'bar',
				data: {
					labels: ['Ventas', 'Compras', 'Abonos CXC'],
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
				Vista por Mes
			</button>
			<button
				onclick={() => { tabActiva = 'vision'; }}
				class="px-4 pb-3 pt-1 font-barlow text-[13px] font-medium border-b-2 transition-colors duration-150 {tabActiva === 'vision' ? 'text-white border-amber' : 'text-white/45 border-transparent hover:text-white/70'}"
			>
				Vista por Año
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

					<!-- ── Selector de mes ── -->
					<div class="-mx-4 md:-mx-6 px-4 md:px-6 py-2.5 border-b border-slate-200/70 bg-surface/60 mb-4 overflow-x-auto">
						<div class="flex gap-1 min-w-max">
							{#each MESES_CORTOS as mes, i}
								{@const esFuturo = i > mesActual}
								{@const esSeleccionado = i === mesSeleccionado}
								<button
									onclick={() => { mesSeleccionado = i; }}
									disabled={esFuturo}
									class="px-3 py-1 rounded-full text-[11px] font-mono font-medium transition-colors
										{esSeleccionado
											? 'bg-navy text-white'
											: esFuturo
												? 'text-slate-300 pointer-events-none'
												: 'text-slate-500 hover:text-navy hover:bg-navy/[0.08]'}"
								>
									{mes}
								</button>
							{/each}
						</div>
					</div>

					<!-- Gráficos mensuales -->
					<p class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-slate-400 animate-fadeSlide" style="animation-delay: 80ms">Gráfica</p>
					<div class="bg-surface rounded p-4 border border-slate-200 animate-fadeSlide" style="animation-delay: 100ms">
						<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-3">
							{MESES_COMPLETOS[mesSeleccionado].toUpperCase()} — {anioActual} vs {anioActual - 1}
						</p>
						<div class="relative h-48 sm:h-56">
							<canvas bind:this={canvasMes}></canvas>
						</div>
					</div>

					<!-- Tabla desglose: 4 columnas unificadas -->
					<p class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-slate-400 animate-fadeSlide" style="animation-delay: 120ms">Desglose</p>
					<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 140ms">

						<!-- Header 3 columnas -->
						<div class="grid grid-cols-3 bg-bg border-b border-slate-100">
							<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2 border-r border-slate-100">
								<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-green pr-2">Ventas</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual}</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual - 1}</p>
							</div>
							<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2 border-r border-slate-100">
								<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-amber pr-2">Gastos</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual}</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual - 1}</p>
							</div>
							<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2">
								<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-violet-600 pr-2">Abonos</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual}</p>
								<p class="text-right text-[9px] font-mono text-slate-400">{anioActual - 1}</p>
							</div>
						</div>

						<!-- Body 3 columnas -->
						<div class="grid grid-cols-3 divide-x divide-slate-100">
							<!-- Col 1: Ventas -->
							<div class="flex flex-col">
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[11px] text-slate-600 pr-2">Facturas</span>
									<span class="font-mono text-[11px] text-green text-right">{formatMXN(mesData.facturas_importe)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(mesPrevData.facturas_importe)}</span>
								</div>
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[11px] text-slate-600 pr-2">Remisiones</span>
									<span class="font-mono text-[11px] text-green text-right">{formatMXN(mesData.remisiones_importe)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(mesPrevData.remisiones_importe)}</span>
								</div>
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[11px] text-slate-600 pr-2">Notas</span>
									<span class="font-mono text-[11px] text-green text-right">{formatMXN(mesData.notas_importe)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(mesPrevData.notas_importe)}</span>
								</div>
								<div class="mt-auto grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-t border-slate-200 bg-green/[0.04] items-center">
									<span class="text-[10px] font-mono font-bold text-slate-500 uppercase tracking-wider pr-2">Total</span>
									<span class="font-mono text-[12px] font-bold text-green text-right">{formatMXN(mesData.ventas_importe)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(mesPrevData.ventas_importe)}</span>
								</div>
							</div>
							<!-- Col 2: Gastos (Compras + Devoluciones) -->
							<div class="flex flex-col">
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[11px] text-slate-600 pr-2">Compras</span>
									<span class="font-mono text-[11px] text-amber text-right">{formatMXN(mesData.compras_importe)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(mesPrevData.compras_importe)}</span>
								</div>
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[11px] text-slate-600 pr-2">Devoluciones</span>
									<span class="font-mono text-[11px] text-orange-600 text-right">{formatMXN(mesData.devoluciones_importe)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(mesPrevData.devoluciones_importe)}</span>
								</div>
								<div class="mt-auto grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-t border-slate-200 bg-amber/[0.04] items-center">
									<span class="text-[10px] font-mono font-bold text-slate-500 uppercase tracking-wider pr-2">Total</span>
									<span class="font-mono text-[12px] font-bold text-amber text-right">{formatMXN(gastosActual)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(gastosPrevio)}</span>
								</div>
							</div>
							<!-- Col 3: Abonos CXC -->
							<div class="flex flex-col">
								<div class="grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-b border-slate-50 items-center">
									<span class="text-[11px] text-slate-600 pr-2">Abonos CXC</span>
									<span class="font-mono text-[11px] text-violet-600 text-right">{formatMXN(mesData.abonos_importe)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(mesPrevData.abonos_importe)}</span>
								</div>
								<div class="mt-auto grid grid-cols-[minmax(0,auto)_1fr_1fr] px-3 py-2.5 border-t border-slate-200 bg-violet-500/[0.04] items-center">
									<span class="text-[10px] font-mono font-bold text-slate-500 uppercase tracking-wider pr-2">Total</span>
									<span class="font-mono text-[12px] font-bold text-violet-600 text-right">{formatMXN(mesData.abonos_importe)}</span>
									<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(mesPrevData.abonos_importe)}</span>
								</div>
							</div>
						</div>

					</div>

					<!-- Resumen: 3 cards -->
					<p class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-slate-400 animate-fadeSlide">Resumen</p>

					<!-- Balance Ventas vs Compras -->
					<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-4 py-3 flex items-center justify-between animate-fadeSlide overflow-hidden"
						style="border-left-color: {balanceActual >= 0 ? '#1f9254' : '#ef4444'}">
						<div>
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-0.5">Balance General</p>
							<p class="text-[9px] font-mono text-slate-400">Ventas + Abonos − Gastos</p>
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

					<!-- Tablas detalle Inventario y CXC -->
					<div class="grid grid-cols-2 gap-3 animate-fadeSlide">

						<!-- Tabla Inventario -->
						<div class="bg-surface rounded border border-slate-200 overflow-hidden">
							<div class="flex items-center justify-between px-3 py-2 bg-bg border-b border-slate-100">
								<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-teal-600">Inventario</p>
								<p class="text-[9px] font-mono text-slate-400">Importe</p>
							</div>
							<div class="flex flex-col">
								<div class="flex justify-between items-center px-3 py-2.5 border-b border-slate-50">
									<span class="text-[11px] text-slate-500">Saldo ini.</span>
									<span class="font-mono text-[11px] text-slate-600">{datosInventario ? formatMXN(invMesData.saldo_inicial) : '—'}</span>
								</div>
								<div class="flex justify-between items-center px-3 py-2.5 border-b border-slate-50">
									<span class="text-[11px] text-slate-500">+ Entradas</span>
									<span class="font-mono text-[11px] text-green">{datosInventario ? '+' + formatMXN(invMesData.entradas) : '—'}</span>
								</div>
								<div class="flex justify-between items-center px-3 py-2.5 border-b border-slate-50">
									<span class="text-[11px] text-slate-500">− Salidas</span>
									<span class="font-mono text-[11px] text-red-500">{datosInventario ? '−' + formatMXN(invMesData.salidas) : '—'}</span>
								</div>
								<div class="flex justify-between items-center px-3 py-2.5 border-t border-slate-200 bg-teal-50/50">
									<span class="text-[10px] font-mono font-bold text-slate-500 uppercase tracking-wider">Saldo final</span>
									<span class="font-mono text-[12px] font-bold text-teal-600">{datosInventario ? formatMXN(invMesData.saldo_final) : '—'}</span>
								</div>
							</div>
						</div>

						<!-- Tabla CXC -->
						<div class="bg-surface rounded border border-slate-200 overflow-hidden">
							<div class="flex items-center justify-between px-3 py-2 bg-bg border-b border-slate-100">
								<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-violet-600">CXC</p>
								<p class="text-[9px] font-mono text-slate-400">Importe</p>
							</div>
							<div class="flex flex-col">
								<div class="flex justify-between items-center px-3 py-2.5 border-b border-slate-50">
									<span class="text-[11px] text-slate-500">Saldo ini.</span>
									<span class="font-mono text-[11px] text-slate-600">{datosCxc ? formatMXN(cxcMesData.saldo_inicial) : '—'}</span>
								</div>
								<div class="flex justify-between items-center px-3 py-2.5 border-b border-slate-50">
									<span class="text-[11px] text-slate-500">+ Cargos</span>
									<span class="font-mono text-[11px] text-green">{datosCxc ? '+' + formatMXN(cxcMesData.cargos) : '—'}</span>
								</div>
								<div class="flex justify-between items-center px-3 py-2.5 border-b border-slate-50">
									<span class="text-[11px] text-slate-500">− Abonos</span>
									<span class="font-mono text-[11px] text-red-500">{datosCxc ? '−' + formatMXN(cxcMesData.abonos) : '—'}</span>
								</div>
								<div class="flex justify-between items-center px-3 py-2.5 border-t border-slate-200 bg-violet-50/50">
									<span class="text-[10px] font-mono font-bold text-slate-500 uppercase tracking-wider">Saldo final</span>
									<span class="font-mono text-[12px] font-bold text-violet-600">{datosCxc ? formatMXN(cxcMesData.saldo_final) : '—'}</span>
								</div>
							</div>
						</div>

					</div>

					<!-- Diferencias Inventario y CXC -->
					<div class="grid grid-cols-2 gap-3 animate-fadeSlide">

						<!-- Diferencia Inventario -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-4 py-3 flex items-center justify-between overflow-hidden"
							style="border-left-color: {!datosInventario ? '#94a3b8' : difInv >= 0 ? '#14b8a6' : '#ef4444'}">
							<div>
								<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-0.5">Inventario</p>
								<p class="text-[9px] font-mono text-slate-400">Entradas − Salidas</p>
							</div>
							<div class="text-right">
								<span class="font-barlow-condensed text-[28px] font-bold leading-none {!datosInventario ? 'text-slate-300' : difInv >= 0 ? 'text-teal-600' : 'text-red-500'}">
									{datosInventario ? formatMXN(difInv) : '—'}
								</span>
							</div>
						</div>

						<!-- Diferencia CXC -->
						<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-4 py-3 flex items-center justify-between overflow-hidden"
							style="border-left-color: {!datosCxc ? '#94a3b8' : difCxc >= 0 ? '#7c3aed' : '#ef4444'}">
							<div>
								<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-0.5">CXC</p>
								<p class="text-[9px] font-mono text-slate-400">Cargos − Abonos</p>
							</div>
							<div class="text-right">
								<span class="font-barlow-condensed text-[28px] font-bold leading-none {!datosCxc ? 'text-slate-300' : difCxc >= 0 ? 'text-violet-600' : 'text-red-500'}">
									{datosCxc ? formatMXN(difCxc) : '—'}
								</span>
							</div>
						</div>

					</div>

				{:else}

					<!-- ── VISIÓN GENERAL ── -->

					<!-- Gráfico tendencia anual -->
					<p class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-slate-400 animate-fadeSlide" style="animation-delay: 40ms">Gráfica</p>
					<div class="bg-surface rounded p-4 border border-slate-200 animate-fadeSlide" style="animation-delay: 60ms">
						<p class="text-[9px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-3">
							TENDENCIA ANUAL — {anioActual} vs {anioActual - 1}
						</p>
						<div class="relative h-48 sm:h-60">
							<canvas bind:this={canvasAnual}></canvas>
						</div>
					</div>

					<!-- Tabla 12 meses -->
					<p class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-slate-400 animate-fadeSlide" style="animation-delay: 80ms">Desglose</p>
					<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 100ms">

						<!-- Cabecera: row de categorías -->
						<div class="grid grid-cols-[1.4fr_1.7fr_1.7fr_1.7fr_0.9fr_0.9fr] bg-bg border-b border-slate-100 px-3 pt-2 pb-0.5 gap-1.5">
							<span></span>
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-green text-right">Ventas</span>
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-amber text-right">Compras</span>
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-violet-500 text-right">Abonos</span>
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-teal-600 text-right">Inventario</span>
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-violet-500 text-right">CXC</span>
						</div>
						<!-- Cabecera: row de años -->
						<div class="grid grid-cols-[1.4fr_0.95fr_0.75fr_0.95fr_0.75fr_0.95fr_0.75fr_0.9fr_0.9fr] bg-bg border-b border-slate-200 px-3 pt-0.5 pb-2 gap-1.5">
							<span class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-slate-400">Mes</span>
							<span class="text-[8px] font-mono text-green/70 text-right">{anioActual}</span>
							<span class="text-[8px] font-mono text-slate-400 text-right">{anioActual - 1}</span>
							<span class="text-[8px] font-mono text-amber/70 text-right">{anioActual}</span>
							<span class="text-[8px] font-mono text-slate-400 text-right">{anioActual - 1}</span>
							<span class="text-[8px] font-mono text-violet-400/70 text-right">{anioActual}</span>
							<span class="text-[8px] font-mono text-slate-400 text-right">{anioActual - 1}</span>
							<span></span>
							<span></span>
						</div>

						<!-- Filas mensuales -->
						{#each filasMeses as fila}
							{@const esActual = fila.idx === mesActual}
							<div
								class="grid grid-cols-[1.4fr_0.95fr_0.75fr_0.95fr_0.75fr_0.95fr_0.75fr_0.9fr_0.9fr] px-3 py-1.5 gap-1.5 border-b border-slate-50 items-center
									{esActual ? 'bg-amber/[0.035] border-l-[3px] border-amber' : 'border-l-[3px] border-transparent'}"
							>
								<span class="text-[12px] {esActual ? 'text-navy font-semibold' : 'text-slate-600'} font-barlow truncate">
									{fila.nombre}
								</span>
								<div class="flex flex-col items-end">
									<span class="font-mono text-[11px] text-navy">{formatMXN(fila.act.ventas_importe)}</span>
									<span class="font-mono text-[9px] {deltaClass(fila.deltaV)}">{fmtDelta(fila.deltaV)}</span>
								</div>
								<span class="font-mono text-[10px] text-slate-400 text-right">{formatMXN(fila.ant.ventas_importe)}</span>
								<div class="flex flex-col items-end">
									<span class="font-mono text-[11px] text-navy">{formatMXN(fila.act.compras_importe)}</span>
									<span class="font-mono text-[9px] {deltaClass(fila.deltaC)}">{fmtDelta(fila.deltaC)}</span>
								</div>
								<span class="font-mono text-[10px] text-slate-400 text-right">{formatMXN(fila.ant.compras_importe)}</span>
								<div class="flex flex-col items-end">
									<span class="font-mono text-[11px] text-navy">{formatMXN(fila.act.abonos_importe)}</span>
									<span class="font-mono text-[9px] {deltaClass(fila.deltaA)}">{fmtDelta(fila.deltaA)}</span>
								</div>
								<span class="font-mono text-[10px] text-slate-400 text-right">{formatMXN(fila.ant.abonos_importe)}</span>
								<span class="font-mono text-[11px] text-right {fila.difInv === null ? 'text-slate-300' : fila.difInv >= 0 ? 'text-teal-600' : 'text-red-500'}">
									{fila.difInv !== null ? formatMXN(fila.difInv) : '—'}
								</span>
								<span class="font-mono text-[11px] text-right {fila.difCxc === null ? 'text-slate-300' : fila.difCxc >= 0 ? 'text-violet-600' : 'text-red-500'}">
									{fila.difCxc !== null ? formatMXN(fila.difCxc) : '—'}
								</span>
							</div>
						{/each}

						<!-- Fila de totales -->
						<div class="grid grid-cols-[1.4fr_0.95fr_0.75fr_0.95fr_0.75fr_0.95fr_0.75fr_0.9fr_0.9fr] px-3 py-2.5 gap-1.5 items-center bg-navy/[0.05] border-t-2 border-navy/20">
							<span class="text-[10px] font-mono font-bold text-slate-500 uppercase tracking-wider">Total</span>
							<div class="flex flex-col items-end">
								<span class="font-mono text-[12px] font-bold text-green">{formatMXN(datosActual.total_ventas)}</span>
								<span class="font-mono text-[9px] {deltaClass(deltaVentasAnual)}">{fmtDelta(deltaVentasAnual)}</span>
							</div>
							<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(datosAnioAnterior?.total_ventas ?? 0)}</span>
							<div class="flex flex-col items-end">
								<span class="font-mono text-[12px] font-bold text-amber">{formatMXN(datosActual.total_compras)}</span>
								<span class="font-mono text-[9px] {deltaClass(deltaComprasAnual)}">{fmtDelta(deltaComprasAnual)}</span>
							</div>
							<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(datosAnioAnterior?.total_compras ?? 0)}</span>
							<div class="flex flex-col items-end">
								<span class="font-mono text-[12px] font-bold text-violet-600">{formatMXN(datosActual.total_abonos)}</span>
								<span class="font-mono text-[9px] {deltaClass(deltaAbonosAnual)}">{fmtDelta(deltaAbonosAnual)}</span>
							</div>
							<span class="font-mono text-[11px] text-slate-400 text-right">{formatMXN(datosAnioAnterior?.total_abonos ?? 0)}</span>
							<span class="font-mono text-[12px] font-bold text-right {totalInvAnual === null ? 'text-slate-300' : totalInvAnual >= 0 ? 'text-teal-600' : 'text-red-500'}">
								{totalInvAnual !== null ? formatMXN(totalInvAnual) : '—'}
							</span>
							<span class="font-mono text-[12px] font-bold text-right {totalCxcAnual === null ? 'text-slate-300' : totalCxcAnual >= 0 ? 'text-violet-600' : 'text-red-500'}">
								{totalCxcAnual !== null ? formatMXN(totalCxcAnual) : '—'}
							</span>
						</div>

					</div>

					<!-- Balance General anual -->
					<p class="text-[9px] font-mono font-bold tracking-[0.14em] uppercase text-slate-400 animate-fadeSlide" style="animation-delay: 120ms">Resumen</p>
					<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-4 py-3 flex items-center justify-between animate-fadeSlide overflow-hidden"
						style="animation-delay: 140ms; border-left-color: {balAnual >= 0 ? '#1f9254' : '#ef4444'}">
						<div>
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-0.5">Balance General {anioActual}</p>
							<p class="text-[9px] font-mono text-slate-400">Ventas + Abonos − Gastos</p>
						</div>
						<div class="text-right">
							<span class="font-barlow-condensed text-[34px] font-bold leading-none {balAnual >= 0 ? 'text-green' : 'text-red-500'}">
								{formatMXN(balAnual)}
							</span>
							{#if deltaBalAnual !== null}
								<p class="text-[9px] font-mono {deltaBalAnual >= 0 ? 'text-green' : 'text-red-400'} mt-0.5">
									{deltaBalAnual >= 0 ? '↑' : '↓'} {Math.abs(deltaBalAnual).toFixed(1)}% vs {anioActual - 1}
								</p>
							{/if}
						</div>
					</div>

				{/if}

			</div>
			{/key}

		{/if}

	</div>
</div>
