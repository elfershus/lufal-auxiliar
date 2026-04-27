import { invoke } from '@tauri-apps/api/core';
import type { ArticuloFracciones, ArticuloSearchResult, DbfPaths, Etiqueta } from './types.js';

export interface FraccionesInitData {
	fracciones: ArticuloFracciones[];
	articulos: ArticuloSearchResult[];
	etiquetas: Etiqueta[];
}

export function getDbfPaths(): Promise<DbfPaths> {
	return invoke('get_dbf_paths');
}

export function saveDbfArts(path: string): Promise<void> {
	return invoke('save_dbf_arts', { path });
}

export function saveDbfUnidades(path: string): Promise<void> {
	return invoke('save_dbf_unidades', { path });
}

export function getFraccionesInitData(): Promise<FraccionesInitData> {
	return invoke('get_fracciones_init_data');
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
