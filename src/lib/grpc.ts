import { invoke } from '@tauri-apps/api/core';
import type {
	AlmacenRecord,
	ArticuloInfo,
	ListDocumentosParams,
	ListDocumentosResult,
	GetDocumentoResult,
	SeguimientoResult,
} from './types';

export async function listAlmacenes(): Promise<AlmacenRecord[]> {
	return invoke<AlmacenRecord[]>('list_almacenes');
}

export async function listDocumentos(
	params: ListDocumentosParams
): Promise<ListDocumentosResult> {
	return invoke<ListDocumentosResult>('list_documentos', { params });
}

export async function getDocumento(
	tipodoc: string,
	numdoc: string
): Promise<GetDocumentoResult> {
	return invoke<GetDocumentoResult>('get_documento', { tipodoc, numdoc });
}

export async function getProveedorNombre(numprov: string): Promise<string> {
	return invoke<string>('get_proveedor_nombre', { numprov });
}

export async function buscarSeguimiento(
	numarts: string[],
	fechaDesde: string,
	numalm: string
): Promise<SeguimientoResult> {
	return invoke<SeguimientoResult>('buscar_seguimiento', { numarts, fechaDesde, numalm });
}

export async function getArticulos(numarts: string[]): Promise<Record<string, ArticuloInfo>> {
	const list = await invoke<ArticuloInfo[]>('get_articulos', { numarts });
	return Object.fromEntries(list.map((a) => [a.numart, a]));
}

export async function getConfigPath(): Promise<string | null> {
	return invoke<string | null>('get_config_path');
}

export async function checkConfig(): Promise<boolean> {
	return invoke<boolean>('check_config');
}

export async function initClient(): Promise<boolean> {
	return invoke<boolean>('init_client');
}

export async function saveConfig(endpoint: string, apiKey: string): Promise<void> {
	return invoke<void>('save_config', { endpoint, apiKey });
}
