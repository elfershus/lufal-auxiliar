import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { ArticuloFracciones, ArticuloSearchResult, CxcMensualAnioResult, SucursalesConfig, EstadisticasResult, EstadisticasDosAniosResult, Etiqueta, InventarioAnioResult, InventarioMesDetalleResult, PairingRow, ParsePairingsResult, SeguimientoFraccionRow, ParseSeguimientosResult } from './types.js';

export interface FraccionesInitData {
	fracciones: ArticuloFracciones[];
	articulos: ArticuloSearchResult[];
	etiquetas: Etiqueta[];
	seguimientos: SeguimientoFraccionRow[];
}

export function getSucursalesConfig(): Promise<SucursalesConfig> {
	return invoke('get_dbf_paths');
}

export function saveSucursalDbfPath(numalm: string, path: string): Promise<void> {
	return invoke('save_sucursal_dbf_path', { numalm, path });
}

export function saveDefaultNumalm(numalm: string): Promise<void> {
	return invoke('save_default_numalm', { numalm });
}

export function saveSucursalesMap(mapping: import('./types.js').SucursalEntry[]): Promise<void> {
	return invoke('save_sucursales_map', { mapping });
}

export function getEstadisticasDocum(
	fechaFrom?: string,
	fechaTo?: string,
	numalm?: string,
): Promise<EstadisticasResult> {
	return invoke('get_estadisticas_docum', { fechaFrom, fechaTo, numalm });
}

export function getEstadisticasDosAnios(
	anio: number,
	numalm?: string,
): Promise<EstadisticasDosAniosResult> {
	return invoke('get_estadisticas_dos_anios', { anio, numalm: numalm ?? null });
}

export function getEstadisticasInventarioDetalle(numalm?: string): Promise<InventarioAnioResult> {
	return invoke('get_estadisticas_inventario_detalle', { numalm: numalm ?? null });
}

export function getEstadisticasCxcMensual(numalm?: string): Promise<CxcMensualAnioResult> {
	return invoke('get_estadisticas_cxc_mensual', { numalm: numalm ?? null });
}

export function getInventarioPorMes(
	numalm: string,
	anio: number,
	mes: number,
): Promise<InventarioMesDetalleResult> {
	return invoke('get_inventario_por_mes', { numalm, anio, mes });
}

export function getFraccionesInitData(numalm?: string): Promise<FraccionesInitData> {
	return invoke('get_fracciones_init_data', { numalm: numalm ?? null });
}

export function saveFraccionPairing(
	numartOrigen: string,
	unidadFraccion: string,
	numartDestino: string
): Promise<void> {
	return invoke('save_fraccion_pairing', { numartOrigen, unidadFraccion, numartDestino });
}

export function deleteFraccionPairing(
	numartOrigen: string,
	unidadFraccion: string
): Promise<void> {
	return invoke('delete_fraccion_pairing', { numartOrigen, unidadFraccion });
}

export function getEtiquetas(): Promise<Etiqueta[]> {
	return invoke('get_etiquetas');
}

export function createEtiqueta(nombre: string, color: string): Promise<Etiqueta> {
	return invoke('create_etiqueta', { nombre, color });
}

export function updateEtiqueta(id: number, nombre: string, color: string): Promise<void> {
	return invoke('update_etiqueta', { id, nombre, color });
}

export function deleteEtiqueta(id: number): Promise<void> {
	return invoke('delete_etiqueta', { id });
}

export function setEmparejamientoEtiquetas(
	numartOrigen: string,
	unidadFraccion: string,
	etiquetaIds: number[]
): Promise<void> {
	return invoke('set_emparejamiento_etiquetas', { numartOrigen, unidadFraccion, etiquetaIds });
}

// ── Seguimientos ──────────────────────────────────────────────

export function addSeguimientoFraccion(numartOrigen: string, unidadFraccion: string): Promise<void> {
	return invoke('add_seguimiento_fraccion', { numartOrigen, unidadFraccion });
}

export function deleteSeguimientoFraccion(numartOrigen: string, unidadFraccion: string): Promise<void> {
	return invoke('delete_seguimiento_fraccion', { numartOrigen, unidadFraccion });
}

export async function downloadSeguimientosTemplate(): Promise<void> {
	const path = await save({
		title: 'Guardar plantilla de seguimientos',
		defaultPath: 'plantilla_seguimientos.xlsx',
		filters: [{ name: 'Excel', extensions: ['xlsx'] }],
	});
	if (!path) return;
	await invoke('export_seguimientos_template', { path });
}

export async function exportSeguimientosXlsx(): Promise<number> {
	const path = await save({
		title: 'Exportar seguimientos',
		defaultPath: 'seguimientos.xlsx',
		filters: [{ name: 'Excel', extensions: ['xlsx'] }],
	});
	if (!path) return 0;
	return invoke('export_seguimientos_xlsx', { path });
}

export async function parseSeguimientosXlsx(): Promise<ParseSeguimientosResult | null> {
	const path = await open({
		title: 'Seleccionar archivo XLSX de seguimientos',
		multiple: false,
		filters: [{ name: 'Excel', extensions: ['xlsx', 'xls'] }],
	});
	if (!path || typeof path !== 'string') return null;
	return invoke('parse_seguimientos_xlsx', { path });
}

export async function importSeguimientos(
	rows: SeguimientoFraccionRow[],
	mode: 'agregar' | 'reemplazar'
): Promise<number> {
	return invoke('import_seguimientos', { rows, mode });
}

// ── XLSX ──────────────────────────────────────────────────────

export async function downloadPairingsTemplate(): Promise<void> {
	const path = await save({
		title: 'Guardar plantilla',
		defaultPath: 'plantilla_emparejamientos.xlsx',
		filters: [{ name: 'Excel', extensions: ['xlsx'] }],
	});
	if (!path) return;
	await invoke('export_pairings_template', { path });
}

export async function exportPairingsXlsx(): Promise<number> {
	const path = await save({
		title: 'Exportar emparejamientos',
		defaultPath: 'emparejamientos.xlsx',
		filters: [{ name: 'Excel', extensions: ['xlsx'] }],
	});
	if (!path) return 0;
	return invoke('export_pairings_xlsx', { path });
}

export async function parsePairingsXlsx(): Promise<ParsePairingsResult | null> {
	const path = await open({
		title: 'Seleccionar archivo XLSX',
		multiple: false,
		filters: [{ name: 'Excel', extensions: ['xlsx', 'xls'] }],
	});
	if (!path || typeof path !== 'string') return null;
	return invoke('parse_pairings_xlsx', { path });
}

export async function importPairings(
	rows: PairingRow[],
	mode: 'agregar' | 'reemplazar'
): Promise<number> {
	return invoke('import_pairings', { rows, mode });
}

// ── IMPRESIÓN ─────────────────────────────────────────────────

export function getDefaultPrinter(): Promise<string | null> {
	return invoke('get_default_printer');
}

export function saveDefaultPrinter(printer: string): Promise<void> {
	return invoke('save_default_printer', { printer });
}
