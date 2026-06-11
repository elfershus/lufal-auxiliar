<script lang="ts">
	import { onDestroy } from 'svelte';
	import {
		Chart, BarController, BarElement,
		LineController, LineElement, PointElement, Filler,
		CategoryScale, LinearScale, Tooltip, Legend
	} from 'chart.js';
	import { getEstadisticasDosAnios, getEstadisticasInventarioDetalle, getEstadisticasCxcMensual, getInventarioPorMes } from '../lib/dbf.js';
	import { listMinvPorArticulo } from '../lib/grpc.js';
	import { appConfig } from '../lib/config.svelte.js';
	import { auth } from '../lib/auth.svelte.js';
	import LoginGate from '../components/LoginGate.svelte';
	import { formatMXN } from '../lib/utils.js';
	import type { EstadisticasResult, PeriodoStat, InventarioAnioResult, CxcMensualAnioResult, InventarioMesStat, CxcMensualMesStat, MinvRecord, InventarioMesDetalleResult } from '../lib/types.js';

	Chart.register(BarController, BarElement, LineController, LineElement, PointElement, Filler, CategoryScale, LinearScale, Tooltip, Legend);

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
	let chartMes: Chart | null = null;

	// Visión General — 4 gráficas de línea
	let canvasVentasContado = $state<HTMLCanvasElement | null>(null);
	let canvasGastos        = $state<HTMLCanvasElement | null>(null);
	let canvasAbonos        = $state<HTMLCanvasElement | null>(null);
	let chartVentasContado: Chart | null = null;
	let chartGastos:        Chart | null = null;
	let chartAbonos:        Chart | null = null;

	// Columna derecha — 3 gráficas
	let canvasBalance    = $state<HTMLCanvasElement | null>(null);
	let canvasInventario = $state<HTMLCanvasElement | null>(null);
	let canvasCxc        = $state<HTMLCanvasElement | null>(null);
	let chartBalance:    Chart | null = null;
	let chartInventario: Chart | null = null;
	let chartCxc:        Chart | null = null;

	// Tab y navegación de mes
	let tabActiva       = $state<'detalles' | 'vision' | 'kardex'>('detalles');
	let mesSeleccionado = $state(mesActual);

	// Kardex
	let canvasEntradaSalida = $state<HTMLCanvasElement | null>(null);
	let chartEntradaSalida: Chart | null = null;
	let kardexArticulo  = $state('');
	let kardexFechaFrom = $state(`${anioActual}-01-01`);
	let kardexFechaTo   = $state(`${anioActual}-12-31`);
	let kardexCargando  = $state(false);
	let kardexError     = $state('');
	let kardexMovs      = $state<MinvRecord[]>([]);
	let kardexNextPage  = $state('');

	// Catálogo por Mes
	let kardexSubTab   = $state<'por-articulo' | 'por-mes'>('por-articulo');
	let catMesAnio     = $state(anioActual);
	let catMesMes      = $state(mesActual + 1);
	let catMesCargando = $state(false);
	let catMesError    = $state('');
	let catMesDatos    = $state<InventarioMesDetalleResult | null>(null);
	let catMesFiltro   = $state('');

	// KPIs kardex — Por Artículo (conservados, sin binding en template)
	const kardexTotalEntradas = $derived(kardexMovs.reduce((s, m) => s + (m.cant > 0 ? m.cant : 0), 0));
	const kardexTotalSalidas  = $derived(kardexMovs.reduce((s, m) => s + (m.cant < 0 ? Math.abs(m.cant) : 0), 0));
	const kardexTotalCosto    = $derived(kardexMovs.reduce((s, m) => s + m.costo, 0));
	const kardexNetoCant      = $derived(kardexTotalEntradas - kardexTotalSalidas);

	// Artículos filtrados por clave o descripción
	const articulosFiltrados = $derived(
		catMesDatos?.articulos.filter(a => {
			const q = catMesFiltro.toLowerCase().trim();
			if (!q) return true;
			return a.numart.toLowerCase().includes(q) || (a.desc ?? '').toLowerCase().includes(q);
		}) ?? []
	);

	// Totales footer — reflejan el filtro activo
	const catMesTotalSaldoInicial = $derived(articulosFiltrados.reduce((s, a) => s + a.saldo_inicial, 0));
	const catMesTotalEntradas     = $derived(articulosFiltrados.reduce((s, a) => s + a.entradas, 0));
	const catMesTotalSalidas      = $derived(articulosFiltrados.reduce((s, a) => s + a.salidas, 0));
	const catMesTotalSaldoFinal   = $derived(articulosFiltrados.reduce((s, a) => s + a.saldo_final, 0));

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

	$effect(() => { if (auth.unlocked) cargar(); });

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
	const saldoFinalInv = $derived((() => {
		const key = `${anioActual}-${String(mesActual + 1).padStart(2, '0')}`;
		return datosInventario?.meses.find(m => m.mes === key)?.saldo_final ?? null;
	})());
	const saldoFinalCxc = $derived((() => {
		const key = `${anioActual}-${String(mesActual + 1).padStart(2, '0')}`;
		return datosCxc?.meses.find(m => m.mes === key)?.saldo_final ?? null;
	})());

	// Valores del mes actual para headers de tarjetas de gráfica
	const mesActualVentasContado = $derived(getPeriodo(datosActual, anioActual, mesActual).ventas_importe);
	const mesActualGastos        = $derived(getPeriodo(datosActual, anioActual, mesActual).compras_importe);
	const mesActualAbonos        = $derived(getPeriodo(datosActual, anioActual, mesActual).abonos_importe);

	// Headers columna derecha
	const mesActualBalance = $derived((() => {
		const p = getPeriodo(datosActual, anioActual, mesActual);
		return p.ventas_importe + p.abonos_importe - p.compras_importe - p.devoluciones_importe;
	})());
	const mesActualInv = $derived((() => {
		const key = `${anioActual}-${String(mesActual + 1).padStart(2, '0')}`;
		return datosInventario?.meses.find(m => m.mes === key)?.saldo_final ?? null;
	})());
	const mesActualCxc = $derived((() => {
		const key = `${anioActual}-${String(mesActual + 1).padStart(2, '0')}`;
		return datosCxc?.meses.find(m => m.mes === key)?.saldo_final ?? null;
	})());

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

	function makeLineConfig(
		currentData: number[],
		prevData: number[] | null,
		borderColor: string,
		fillColor: string,
	): any {
		const currentMasked = currentData.map((v, i) => (i <= mesActual ? v : null));
		return {
			type: 'line',
			data: {
				labels: MESES_CORTOS,
				datasets: [
					{
						label: String(anioActual),
						data: currentMasked,
						borderColor, borderWidth: 2,
						backgroundColor: fillColor,
						fill: 'origin', tension: 0.35, spanGaps: false,
						pointRadius: currentMasked.map((_, i) => (i === mesActual ? 4 : 0)),
						pointHoverRadius: 5,
						pointBackgroundColor: borderColor,
					},
					...(prevData !== null ? [{
						label: String(anioActual - 1),
						data: prevData,
						borderColor: borderColor + '55',
						borderWidth: 1.5, borderDash: [4, 4],
						backgroundColor: 'transparent',
						fill: false, tension: 0.35, spanGaps: true,
						pointRadius: 0, pointHoverRadius: 4,
					}] : []),
				],
			},
			options: {
				responsive: true, maintainAspectRatio: false, animation: false,
				plugins: {
					legend: { display: false },
					tooltip: {
						mode: 'index', intersect: false,
						callbacks: { label(ctx: any) { return ` ${ctx.dataset.label}: ${formatMXN(ctx.parsed.y ?? 0)}`; } },
					},
				},
				scales: {
					x: { grid: { display: false }, ticks: { font: { family: 'JetBrains Mono, monospace', size: 9 }, color: '#94a3b8' } },
					y: {
						beginAtZero: false,
						grid: { color: 'rgba(0,0,0,0.05)' },
						border: { display: false },
						ticks: {
							font: { family: 'JetBrains Mono, monospace', size: 9 }, color: '#94a3b8', maxTicksLimit: 4,
							callback(val: any) {
								const n = Number(val);
								if (n >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`;
								if (n >= 1_000)     return `$${(n / 1_000).toFixed(0)}K`;
								return `$${n}`;
							},
						},
					},
				},
			},
		};
	}

	$effect(() => {
		if (!datosActual || !canvasVentasContado) return;
		const c = new Chart(canvasVentasContado, makeLineConfig(
			MESES_CORTOS.map((_, i) => getPeriodo(datosActual, anioActual, i).ventas_importe),
			MESES_CORTOS.map((_, i) => getPeriodo(datosAnioAnterior, anioActual - 1, i).ventas_importe),
			'#1f9254', 'rgba(31,146,84,0.10)',
		) as any);
		chartVentasContado = c;
		return () => { c.destroy(); };
	});

	$effect(() => {
		if (!datosActual || !canvasGastos) return;
		const c = new Chart(canvasGastos, makeLineConfig(
			MESES_CORTOS.map((_, i) => getPeriodo(datosActual, anioActual, i).compras_importe),
			MESES_CORTOS.map((_, i) => getPeriodo(datosAnioAnterior, anioActual - 1, i).compras_importe),
			'#e8820a', 'rgba(232,130,10,0.10)',
		) as any);
		chartGastos = c;
		return () => { c.destroy(); };
	});

	$effect(() => {
		if (!datosActual || !canvasAbonos) return;
		const c = new Chart(canvasAbonos, makeLineConfig(
			MESES_CORTOS.map((_, i) => getPeriodo(datosActual, anioActual, i).abonos_importe),
			MESES_CORTOS.map((_, i) => getPeriodo(datosAnioAnterior, anioActual - 1, i).abonos_importe),
			'#14b8a6', 'rgba(20,184,166,0.10)',
		) as any);
		chartAbonos = c;
		return () => { c.destroy(); };
	});

	// Balance General mensual
	$effect(() => {
		if (!datosActual || !canvasBalance) return;
		const current = MESES_CORTOS.map((_, i) => {
			const p = getPeriodo(datosActual, anioActual, i);
			return p.ventas_importe + p.abonos_importe - p.compras_importe - p.devoluciones_importe;
		});
		const prev = MESES_CORTOS.map((_, i) => {
			const p = getPeriodo(datosAnioAnterior, anioActual - 1, i);
			return p.ventas_importe + p.abonos_importe - p.compras_importe - p.devoluciones_importe;
		});
		const c = new Chart(canvasBalance, makeLineConfig(current, prev, '#334155', 'rgba(51,65,85,0.10)') as any);
		chartBalance = c;
		return () => { c.destroy(); };
	});

	// Inventario mensual
	$effect(() => {
		if (!canvasInventario) return;
		const current = MESES_CORTOS.map((_, i) => {
			const key = `${anioActual}-${String(i + 1).padStart(2, '0')}`;
			const inv = datosInventario?.meses.find(m => m.mes === key);
			return inv != null ? inv.saldo_final : 0;
		});
		const prev = MESES_CORTOS.map((_, i) => {
			const key = `${anioActual - 1}-${String(i + 1).padStart(2, '0')}`;
			const inv = datosInventario?.meses.find(m => m.mes === key);
			return inv != null ? inv.saldo_final : 0;
		});
		const c = new Chart(canvasInventario, makeLineConfig(current, prev, '#14b8a6', 'rgba(20,184,166,0.10)') as any);
		chartInventario = c;
		return () => { c.destroy(); };
	});

	// CXC mensual
	$effect(() => {
		if (!canvasCxc) return;
		const current = MESES_CORTOS.map((_, i) => {
			const key = `${anioActual}-${String(i + 1).padStart(2, '0')}`;
			const cxc = datosCxc?.meses.find(m => m.mes === key);
			return cxc != null ? cxc.saldo_final : 0;
		});
		const prev = MESES_CORTOS.map((_, i) => {
			const key = `${anioActual - 1}-${String(i + 1).padStart(2, '0')}`;
			const cxc = datosCxc?.meses.find(m => m.mes === key);
			return cxc != null ? cxc.saldo_final : 0;
		});
		const c = new Chart(canvasCxc, makeLineConfig(current, prev, '#7c3aed', 'rgba(124,58,237,0.10)') as any);
		chartCxc = c;
		return () => { c.destroy(); };
	});

	// Kardex — gráfica entradas vs. salidas por mes
	$effect(() => {
		if (!canvasEntradaSalida) return;
		const entradas = MESES_CORTOS.map((_, i) => {
			const key = `${anioActual}-${String(i + 1).padStart(2, '0')}`;
			return datosInventario?.meses.find(m => m.mes === key)?.entradas ?? 0;
		});
		const salidas = MESES_CORTOS.map((_, i) => {
			const key = `${anioActual}-${String(i + 1).padStart(2, '0')}`;
			return datosInventario?.meses.find(m => m.mes === key)?.salidas ?? 0;
		});
		const c = new Chart(canvasEntradaSalida, {
			type: 'bar',
			data: {
				labels: MESES_CORTOS,
				datasets: [
					{
						label: 'Entradas',
						data: entradas,
						backgroundColor: 'rgba(20,184,166,0.75)',
						borderColor: 'rgba(20,184,166,1)',
						borderWidth: 1, borderRadius: 3,
					},
					{
						label: 'Salidas',
						data: salidas,
						backgroundColor: 'rgba(232,130,10,0.75)',
						borderColor: 'rgba(232,130,10,1)',
						borderWidth: 1, borderRadius: 3,
					},
				],
			},
			options: {
				responsive: true, maintainAspectRatio: false, animation: false,
				plugins: {
					legend: {
						display: true, position: 'top', align: 'end',
						labels: { font: { family: 'JetBrains Mono, monospace', size: 10 }, color: '#94a3b8', boxWidth: 10, boxHeight: 10, padding: 8, usePointStyle: true, pointStyle: 'rect' },
					},
					tooltip: {
						callbacks: { label(ctx: any) { return ` ${ctx.dataset.label}: ${formatMXN(ctx.parsed.y ?? 0)}`; } },
					},
				},
				scales: {
					x: { grid: { display: false }, ticks: { font: { family: 'JetBrains Mono, monospace', size: 9 }, color: '#94a3b8' } },
					y: {
						beginAtZero: true,
						grid: { color: 'rgba(0,0,0,0.05)' },
						ticks: {
							font: { family: 'JetBrains Mono, monospace', size: 9 }, color: '#94a3b8', maxTicksLimit: 4,
							callback(val: any) {
								const n = Number(val);
								if (n >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`;
								if (n >= 1_000) return `$${(n / 1_000).toFixed(0)}K`;
								return `$${n}`;
							},
						},
					},
				},
			},
		} as any);
		chartEntradaSalida = c;
		return () => { c.destroy(); };
	});

	// Auto-carga al cambiar mes, año o almacén mientras el tab Movimientos está activo
	$effect(() => {
		if (tabActiva !== 'kardex') return;
		const anio = catMesAnio;
		const mes  = catMesMes;
		const alm  = appConfig.numalm;
		if (!alm) return;
		void anio; void mes;
		cargarCatMes();
	});

	async function buscarKardex(resetear = true) {
		if (!kardexArticulo.trim()) return;
		kardexCargando = true;
		kardexError = '';
		if (resetear) { kardexMovs = []; kardexNextPage = ''; }
		try {
			const numalm = appConfig.numalm || undefined;
			const res = await listMinvPorArticulo(
				kardexArticulo.trim(),
				numalm,
				kardexFechaFrom || undefined,
				kardexFechaTo || undefined,
				50,
				resetear ? '' : kardexNextPage,
			);
			kardexMovs = resetear ? res.records : [...kardexMovs, ...res.records];
			kardexNextPage = res.next_page_token;
		} catch (e) {
			kardexError = e instanceof Error ? e.message : String(e);
		} finally {
			kardexCargando = false;
		}
	}

	async function cargarCatMes() {
		if (!appConfig.numalm) return;
		catMesCargando = true;
		catMesError = '';
		catMesDatos = null;
		try {
			catMesDatos = await getInventarioPorMes(appConfig.numalm, catMesAnio, catMesMes);
		} catch (e) {
			catMesError = e instanceof Error ? e.message : String(e);
		} finally {
			catMesCargando = false;
		}
	}

	function fmtCant(val: number): string {
		if (Math.abs(val) < 1e-6) return '—';
		return new Intl.NumberFormat('es-MX', { minimumFractionDigits: 0, maximumFractionDigits: 3 }).format(val);
	}

	interface TipodocBadge { label: string; bg: string; text: string; }
	function tipodocBadge(tipodoc: string): TipodocBadge {
		switch (tipodoc.toUpperCase()) {
			case 'C':  return { label: 'Compra',   bg: 'bg-teal-50',   text: 'text-teal-700' };
			case 'R':  return { label: 'Remisión', bg: 'bg-orange-50',  text: 'text-orange-600' };
			case 'F':  return { label: 'Factura',  bg: 'bg-green-50',  text: 'text-green' };
			case 'N':  return { label: 'Nota',     bg: 'bg-slate-100', text: 'text-slate-600' };
			case 'DN': return { label: 'Dev.Nota', bg: 'bg-violet-50', text: 'text-violet-600' };
			case 'DR': return { label: 'Dev.Rem.', bg: 'bg-orange-50', text: 'text-orange-600' };
			default:   return { label: tipodoc,    bg: 'bg-slate-100', text: 'text-slate-500' };
		}
	}

	onDestroy(() => {
		auth.lock();
		chartMes?.destroy();
		chartVentasContado?.destroy();
		chartGastos?.destroy();
		chartAbonos?.destroy();
		chartBalance?.destroy();
		chartInventario?.destroy();
		chartCxc?.destroy();
		chartEntradaSalida?.destroy();
	});

	function fmtDelta(d: number | null): string {
		if (d === null) return '—';
		return (d >= 0 ? '↑ ' : '↓ ') + Math.abs(d).toFixed(1) + '%';
	}
	function deltaClass(d: number | null): string {
		if (d === null) return 'text-slate-300';
		return d >= 0 ? 'text-green' : 'text-red-500';
	}
	function fmtCompact(val: number): string {
		if (val >= 1_000_000) return `$${(val / 1_000_000).toFixed(2)}M`;
		if (val >= 1_000)     return `$${(val / 1_000).toFixed(0)}K`;
		return formatMXN(val);
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
				disabled={!auth.unlocked || cargando}
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
			<button
				onclick={() => { tabActiva = 'kardex'; }}
				class="px-4 pb-3 pt-1 font-barlow text-[13px] font-medium border-b-2 transition-colors duration-150 {tabActiva === 'kardex' ? 'text-white border-amber' : 'text-white/45 border-transparent hover:text-white/70'}"
			>
				Movimientos
			</button>
		</div>
	</div>
	<!-- Separador gradiente bajo el header -->
	<div class="h-px" style="background: linear-gradient(90deg, rgba(232,130,10,0.35) 0%, rgba(226,232,240,0.8) 40%, transparent 100%)"></div>

	<LoginGate subtitle="Ingresa la contraseña para ver las estadísticas">
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

					<!-- Balance mes seleccionado año anterior -->
					<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-4 py-3 flex items-center justify-between animate-fadeSlide overflow-hidden"
						style="border-left-color: {balancePrevio >= 0 ? '#1f9254' : '#ef4444'}">
						<div>
							<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400 mb-0.5">Balance General {anioActual - 1}</p>
							<p class="text-[9px] font-mono text-slate-400">Ventas + Abonos − Gastos</p>
						</div>
						<div class="text-right">
							<span class="font-barlow-condensed text-[28px] font-bold leading-none {balancePrevio >= 0 ? 'text-green' : 'text-red-500'}">
								{formatMXN(balancePrevio)}
							</span>
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

				{:else if tabActiva === 'vision'}

					<!-- ── VISIÓN GENERAL — grid 2 columnas ── -->
					<div class="grid grid-cols-2 gap-3">

						<!-- ══ COLUMNA IZQUIERDA: 4 tarjetas de gráficas ══ -->
						<div class="flex flex-col gap-3">

							<!-- Ventas Contado -->
							<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 40ms">
								<div class="flex items-center gap-2 px-3 pt-2.5 pb-0">
									<div class="w-[3px] h-5 rounded-full flex-shrink-0" style="background:#1f9254"></div>
									<span class="text-[11px] font-mono font-semibold text-slate-600 flex-1 truncate">Ventas Contado</span>
									<span class="font-barlow-condensed text-[14px] font-bold whitespace-nowrap" style="color:#1f9254">{formatMXN(datosActual.total_ventas)}</span>
									<span class="text-[10px] font-mono text-slate-400 whitespace-nowrap">{MESES_CORTOS[mesActual]}·{fmtCompact(mesActualVentasContado)}</span>
								</div>
								<div class="relative h-[176px] px-1 pb-1">
									<canvas bind:this={canvasVentasContado}></canvas>
								</div>
							</div>

							<!-- Gastos -->
							<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 80ms">
								<div class="flex items-center gap-2 px-3 pt-2.5 pb-0">
									<div class="w-[3px] h-5 rounded-full flex-shrink-0" style="background:#e8820a"></div>
									<span class="text-[11px] font-mono font-semibold text-slate-600 flex-1 truncate">Gastos</span>
									<span class="font-barlow-condensed text-[14px] font-bold whitespace-nowrap" style="color:#e8820a">{formatMXN(datosActual.total_compras)}</span>
									<span class="text-[10px] font-mono text-slate-400 whitespace-nowrap">{MESES_CORTOS[mesActual]}·{fmtCompact(mesActualGastos)}</span>
								</div>
								<div class="relative h-[176px] px-1 pb-1">
									<canvas bind:this={canvasGastos}></canvas>
								</div>
							</div>

							<!-- Abonos -->
							<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 100ms">
								<div class="flex items-center gap-2 px-3 pt-2.5 pb-0">
									<div class="w-[3px] h-5 rounded-full flex-shrink-0" style="background:#14b8a6"></div>
									<span class="text-[11px] font-mono font-semibold text-slate-600 flex-1 truncate">Abonos</span>
									<span class="font-barlow-condensed text-[14px] font-bold whitespace-nowrap" style="color:#14b8a6">{formatMXN(datosActual.total_abonos)}</span>
									<span class="text-[10px] font-mono text-slate-400 whitespace-nowrap">{MESES_CORTOS[mesActual]}·{fmtCompact(mesActualAbonos)}</span>
								</div>
								<div class="relative h-[176px] px-1 pb-1">
									<canvas bind:this={canvasAbonos}></canvas>
								</div>
							</div>

							<!-- Balance General anual -->
							<div class="bg-surface rounded border border-slate-200 border-l-[3px] px-4 py-3 flex items-center justify-between animate-fadeSlide overflow-hidden"
								style="animation-delay: 120ms; border-left-color: {balAnual >= 0 ? '#1f9254' : '#ef4444'}">
								<div>
									<p class="text-[8px] font-mono font-bold tracking-[0.16em] uppercase text-slate-400">Balance General {anioActual}</p>
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

						</div>
						<!-- FIN COLUMNA IZQUIERDA -->

						<!-- ══ COLUMNA DERECHA: 3 gráficas ══ -->
						<div class="flex flex-col gap-3">

							<!-- Balance General -->
							<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 40ms">
								<div class="flex items-center gap-2 px-3 pt-2.5 pb-0">
									<div class="w-[3px] h-5 rounded-full flex-shrink-0" style="background:#334155"></div>
									<span class="text-[11px] font-mono font-semibold text-slate-600 flex-1 truncate">Balance General</span>
									<span class="font-barlow-condensed text-[14px] font-bold whitespace-nowrap {balAnual >= 0 ? 'text-green' : 'text-red-500'}">{formatMXN(balAnual)}</span>
									<span class="text-[10px] font-mono text-slate-400 whitespace-nowrap">{MESES_CORTOS[mesActual]}·{fmtCompact(mesActualBalance)}</span>
								</div>
								<div class="relative h-[176px] px-1 pb-1">
									<canvas bind:this={canvasBalance}></canvas>
								</div>
							</div>

							<!-- Inventario -->
							<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 60ms">
								<div class="flex items-center gap-2 px-3 pt-2.5 pb-0">
									<div class="w-[3px] h-5 rounded-full flex-shrink-0" style="background:#14b8a6"></div>
									<span class="text-[11px] font-mono font-semibold text-slate-600 flex-1 truncate">Inventario</span>
									<span class="font-barlow-condensed text-[14px] font-bold whitespace-nowrap {(saldoFinalInv ?? 0) >= 0 ? 'text-teal-600' : 'text-red-500'}">{saldoFinalInv !== null ? formatMXN(saldoFinalInv) : '—'}</span>
									<span class="text-[10px] font-mono text-slate-400 whitespace-nowrap">{MESES_CORTOS[mesActual]}·{mesActualInv !== null ? fmtCompact(mesActualInv) : '—'}</span>
								</div>
								<div class="relative h-[176px] px-1 pb-1">
									<canvas bind:this={canvasInventario}></canvas>
								</div>
							</div>

							<!-- CXC -->
							<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide" style="animation-delay: 80ms">
								<div class="flex items-center gap-2 px-3 pt-2.5 pb-0">
									<div class="w-[3px] h-5 rounded-full flex-shrink-0" style="background:#7c3aed"></div>
									<span class="text-[11px] font-mono font-semibold text-slate-600 flex-1 truncate">Cuentas por Cobrar</span>
									<span class="font-barlow-condensed text-[14px] font-bold whitespace-nowrap {(saldoFinalCxc ?? 0) >= 0 ? 'text-violet-600' : 'text-red-500'}">{saldoFinalCxc !== null ? formatMXN(saldoFinalCxc) : '—'}</span>
									<span class="text-[10px] font-mono text-slate-400 whitespace-nowrap">{MESES_CORTOS[mesActual]}·{mesActualCxc !== null ? fmtCompact(mesActualCxc) : '—'}</span>
								</div>
								<div class="relative h-[176px] px-1 pb-1">
									<canvas bind:this={canvasCxc}></canvas>
								</div>
							</div>

						</div>
						<!-- FIN COLUMNA DERECHA -->

					</div>
					<!-- FIN VISIÓN GENERAL GRID -->

				{:else if tabActiva === 'kardex'}

					<!-- Navegador año + mes (barra bleed-out, igual a Vista por Mes) -->
					<div class="-mx-4 md:-mx-6 px-4 md:px-6 py-2.5 border-b border-slate-200/70 bg-surface/60 mb-4 overflow-x-auto">
						<div class="flex gap-2 items-center min-w-max">

							<!-- Navegador de año -->
							<div class="flex items-center gap-1 pr-2 border-r border-slate-200">
								<button
									onclick={() => { catMesAnio -= 1; }}
									class="w-6 h-6 flex items-center justify-center rounded text-slate-500 hover:bg-slate-100 text-[13px] leading-none transition-colors"
								>&#8249;</button>
								<span class="font-mono text-[12px] font-bold text-navy tabular-nums w-10 text-center">{catMesAnio}</span>
								<button
									onclick={() => { if (catMesAnio < anioActual) catMesAnio += 1; }}
									disabled={catMesAnio >= anioActual}
									class="w-6 h-6 flex items-center justify-center rounded text-slate-500 hover:bg-slate-100 text-[13px] leading-none transition-colors disabled:opacity-30 disabled:pointer-events-none"
								>&#8250;</button>
							</div>

							<!-- Pills de mes -->
							{#each MESES_CORTOS as mes, i}
								{@const mesNum = i + 1}
								{@const esFuturo = catMesAnio === anioActual && mesNum > mesActual + 1}
								{@const esSeleccionado = mesNum === catMesMes}
								<button
									onclick={() => { catMesMes = mesNum; }}
									disabled={esFuturo}
									class="px-3 py-1 rounded-full text-[11px] font-mono font-medium transition-colors
										{esSeleccionado ? 'bg-navy text-white' : esFuturo ? 'text-slate-300 pointer-events-none' : 'text-slate-500 hover:text-navy hover:bg-navy/[0.08]'}"
								>
									{mes}
								</button>
							{/each}

							<!-- Indicador de estado -->
							{#if catMesCargando}
								<span class="text-[10px] font-mono text-slate-400 pl-2">Cargando…</span>
							{:else if catMesDatos}
								<span class="text-[10px] font-mono text-slate-400 pl-2">
									{catMesDatos.articulos.length} artículo{catMesDatos.articulos.length !== 1 ? 's' : ''}
								</span>
							{:else if !appConfig.numalm}
								<span class="text-[10px] font-mono text-slate-400 pl-2">Configura un almacén primero</span>
							{/if}

						</div>
					</div>

					<!-- Filtro de tabla -->
					{#if catMesDatos && catMesDatos.articulos.length > 0}
						<div class="relative mb-3 animate-fadeSlide">
							<input
								type="text"
								bind:value={catMesFiltro}
								placeholder="Filtrar por clave o descripción…"
								class="w-full h-8 pl-8 pr-3 rounded border border-slate-200 bg-white text-[12px] font-mono text-navy placeholder-slate-300 focus:outline-none focus:ring-1 focus:ring-teal-400"
							/>
							<svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-slate-400 pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
							</svg>
						</div>
					{/if}

					<!-- Error -->
					{#if catMesError}
						<div class="bg-red-50 border border-red-200 rounded px-3 py-2 text-[12px] text-red-600 font-mono animate-fadeSlide mb-3">
							{catMesError}
						</div>
					{/if}

					<!-- Tabla de artículos -->
					{#if articulosFiltrados.length > 0}
						<div class="bg-surface rounded border border-slate-200 overflow-hidden animate-fadeSlide">
							<div class="overflow-x-auto">
								<table class="w-full text-[11px] font-mono">
									<thead>
										<tr class="border-b border-slate-200 bg-bg">
											<th class="px-3 py-2 text-left font-semibold text-slate-500 whitespace-nowrap">Clave</th>
											<th class="px-3 py-2 text-left font-semibold text-slate-500">Descripción</th>
											<th class="px-3 py-2 text-right font-semibold text-slate-500 whitespace-nowrap">Inv. Inicial</th>
											<th class="px-3 py-2 text-right font-semibold text-teal-600 whitespace-nowrap">Entradas</th>
											<th class="px-3 py-2 text-right font-semibold text-orange-500 whitespace-nowrap">Salidas</th>
											<th class="px-3 py-2 text-right font-semibold text-navy whitespace-nowrap">Inv. Final</th>
										</tr>
									</thead>
									<tbody>
										{#each articulosFiltrados as art}
											<tr class="border-b border-slate-50 hover:bg-slate-50/60 transition-colors">
												<td class="px-3 py-1.5 text-slate-600 font-semibold whitespace-nowrap">{art.numart}</td>
												<td class="px-3 py-1.5 text-slate-500 max-w-[220px] truncate">{art.desc || '—'}</td>
												<td class="px-3 py-1.5 text-right text-slate-600 whitespace-nowrap">{fmtCant(art.saldo_inicial)}</td>
												<td class="px-3 py-1.5 text-right whitespace-nowrap {art.entradas > 0 ? 'text-teal-600' : 'text-slate-400'}">{fmtCant(art.entradas)}</td>
												<td class="px-3 py-1.5 text-right whitespace-nowrap {art.salidas > 0 ? 'text-orange-500' : 'text-slate-400'}">{fmtCant(art.salidas)}</td>
												<td class="px-3 py-1.5 text-right font-semibold whitespace-nowrap {art.saldo_final >= 0 ? 'text-navy' : 'text-red-500'}">{fmtCant(art.saldo_final)}</td>
											</tr>
										{/each}
									</tbody>
									<tfoot>
										<tr class="border-t border-slate-200 bg-bg">
											<td class="px-3 py-2 text-[10px] font-mono font-bold tracking-[0.1em] uppercase text-slate-500" colspan="2">
												{catMesFiltro.trim() ? `${articulosFiltrados.length} resultado${articulosFiltrados.length !== 1 ? 's' : ''}` : `Totales · ${articulosFiltrados.length} artículos`}
											</td>
											<td class="px-3 py-2 text-right font-bold text-slate-600 whitespace-nowrap">{fmtCant(catMesTotalSaldoInicial)}</td>
											<td class="px-3 py-2 text-right font-bold text-teal-600 whitespace-nowrap">{fmtCant(catMesTotalEntradas)}</td>
											<td class="px-3 py-2 text-right font-bold text-orange-500 whitespace-nowrap">{fmtCant(catMesTotalSalidas)}</td>
											<td class="px-3 py-2 text-right font-bold whitespace-nowrap {catMesTotalSaldoFinal >= 0 ? 'text-navy' : 'text-red-500'}">{fmtCant(catMesTotalSaldoFinal)}</td>
										</tr>
									</tfoot>
								</table>
							</div>
						</div>
					{:else if catMesDatos}
						<div class="text-center text-[12px] font-mono text-slate-400 py-8 animate-fadeSlide">
							{#if catMesFiltro.trim()}
								Sin artículos que coincidan con <span class="font-semibold text-slate-500">"{catMesFiltro}"</span>
							{:else}
								Sin movimientos en {MESES_COMPLETOS[catMesMes - 1]} {catMesAnio}
							{/if}
						</div>
					{/if}

				{/if}

			</div>
			{/key}

		{/if}

	</div>
	</LoginGate>
</div>
