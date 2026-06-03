export interface AlmacenRecord {
	numalm: string;
	nomalm: string;
	letra: string;
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
	precio1: number;
	precio2: number;
	precio3: number;
	precio4: number;
	precio5: number;
	fracciones: FraccionRecord[];
}

export interface SucursalEntry {
	numalm: string;
	letra: string;
	dbf_path: string | null;
}

export interface SucursalesConfig {
	sucursales: SucursalEntry[];
	default_numalm: string | null;
}

export interface PeriodoStat {
	periodo: string;
	ventas_importe: number;
	compras_importe: number;
	ventas_count: number;
	compras_count: number;
	facturas_importe: number;
	facturas_count: number;
	remisiones_importe: number;
	remisiones_count: number;
	notas_importe: number;
	notas_count: number;
	credito_importe: number;
	credito_count: number;
	abonos_importe: number;
	abonos_count: number;
	devoluciones_importe: number;
	devoluciones_count: number;
}

export interface EstadisticasResult {
	periodos: PeriodoStat[];
	total_ventas: number;
	total_compras: number;
	total_ventas_count: number;
	total_compras_count: number;
	total_credito: number;
	total_credito_count: number;
	total_abonos: number;
	total_abonos_count: number;
	total_devoluciones: number;
	total_devoluciones_count: number;
}

export interface PairingRow {
	numart_origen: string;
	unidad_fraccion: string;
	numart_destino: string;
}

export interface PairingRowPreview {
	row_index: number;
	numart_origen: string;
	unidad_fraccion: string;
	numart_destino: string;
	errors: string[];
}

export interface ParsePairingsResult {
	rows: PairingRowPreview[];
	total_rows: number;
	valid_count: number;
	error_count: number;
}

export interface SeguimientoFraccionRow {
	numart_origen: string;
	unidad_fraccion: string;
}

export interface SeguimientoFraccionPreview {
	row_index: number;
	numart_origen: string;
	unidad_fraccion: string;
	errors: string[];
}

export interface ParseSeguimientosResult {
	rows: SeguimientoFraccionPreview[];
	total_rows: number;
	valid_count: number;
	error_count: number;
}

export interface VerifNivel {
	nivel: number;
	precioActual: number;
	precioMinimo: number;
	diferencia: number;
	desactualizado: boolean;
}

export interface VerifFraccion {
	factor: number;
	niveles: VerifNivel[];
	hayProblema: boolean;
	nivelesConProblema: number[];
}

export interface ArticuloEtiqueta {
	numart: string;
	desc: string;
	codigo: string;
}

export interface ListArticulosEtiquetaResult {
	articulos: ArticuloEtiqueta[];
	next_page_token: string;
}

export interface EstadisticasDosAniosResult {
	actual: EstadisticasResult;
	anterior: EstadisticasResult;
}

export interface InventarioMesStat {
	mes: string;
	saldo_inicial: number;
	entradas: number;
	salidas: number;
	saldo_final: number;
}

export interface InventarioAnioResult {
	meses: InventarioMesStat[];
}

export interface CxcMensualMesStat {
	mes: string;
	saldo_inicial: number;
	cargos: number;
	abonos: number;
	saldo_final: number;
}

export interface CxcMensualAnioResult {
	meses: CxcMensualMesStat[];
}

export interface MinvRecord {
	tipodoc: string;
	numdoc: string;
	numpar: string;
	numart: string;
	fecha: string;
	numalm: string;
	cant: number;
	costo: number;
	costodls: number;
	numprov: string;
	numcli: string;
	refer: string;
	idmotivo: string;
}

export interface ListMinvResult {
	records: MinvRecord[];
	next_page_token: string;
}

export interface ArticuloMovMesStat {
	numart: string;
	desc: string;
	saldo_inicial: number;
	entradas: number;
	salidas: number;
	saldo_final: number;
}

export interface InventarioMesDetalleResult {
	anio: number;
	mes: number;
	articulos: ArticuloMovMesStat[];
}
