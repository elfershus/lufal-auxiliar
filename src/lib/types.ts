export interface AlmacenRecord {
	numalm: string;
	nomalm: string;
}

export interface DocumentoRecord {
	tipodoc: string;
	numdoc: string;
	numalm: string;
	fecha: string;
	numprov: string;
	refer: string;
	importe: number;
	descuento: number;
	impuesto1: number;
	status: number;
	fechacapt: string;
	formapago: string;
	pjedesc: number;
	fechapago: string;
	uuid: string;
	costo: number;
}

export interface MovimientoRecord {
	tipodoc: string;
	numdoc: string;
	numpar: string;
	numart: string;
	precio: number;
	cant: number;
	pend: number;
	pjedesc: number;
	impuesto1: number;
	impuesto2: number;
	unidad: string;
	docant: string;
}

export interface ListDocumentosParams {
	tipodoc?: string;
	numalm?: string;
	fecha_from?: string;
	fecha_to?: string;
	fechacapt_from?: string;
	fechacapt_to?: string;
	numdoc?: string;
	status?: number;
	page_size?: number;
	page_token?: string;
	order_by?: string;
}

export interface CompraMatchItem {
	tipodoc: string;
	numdoc: string;
	fecha: string;
	numprov: string;
	importe: number;
	status: number;
	arts_matched: number;
	total_arts: number;
	coverage_pct: number;
}

export interface RemisionMatchItem {
	tipodoc: string;
	numdoc: string;
	fecha: string;
	numcli: string;
	importe: number;
	status: number;
	arts_matched: number;
	total_arts: number;
	coverage_pct: number;
}

export interface SeguimientoResult {
	compras: CompraMatchItem[];
	remisiones: RemisionMatchItem[];
}

export interface ListDocumentosResult {
	documentos: DocumentoRecord[];
	next_page_token: string;
}

export interface GetDocumentoResult {
	documento: DocumentoRecord;
	movimientos: MovimientoRecord[];
}

export interface ArticuloInfo {
	numart: string;
	desc: string;
	unidad: string;
}

export interface ArticuloPareado {
	numart: string;
	desc: string;
	unidad: string;
	precio1: number;
	precio2: number;
	precio3: number;
	precio4: number;
	precio5: number;
}

export interface ArticuloSearchResult {
	numart: string;
	desc: string;
	unidad: string;
}

export interface Etiqueta {
	id: number;
	nombre: string;
	color: string;
}

export interface FraccionRecord {
	unidad: string;
	equiv1: number;
	equiv2: number;
	precio1: number;
	precio2: number;
	precio3: number;
	precio4: number;
	precio5: number;
	pareado: ArticuloPareado | null;
	etiquetas: Etiqueta[];
}

export interface ArticuloFracciones {
	numart: string;
	desc: string;
	unidad_base: string;
	fracciones: FraccionRecord[];
}

export interface DbfPaths {
	dbf_arts: string | null;
	dbf_unidades: string | null;
}
