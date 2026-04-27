export function formatFecha(iso: string): string {
	if (!iso) return '—';
	const [y, m, d] = iso.split('-');
	const meses = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
	return `${d}/${meses[parseInt(m) - 1]}/${y.slice(2)}`;
}

export function formatFechaCorta(iso: string): string {
	if (!iso) return '';
	const date = new Date(iso + 'T12:00:00');
	const dias = ['Dom', 'Lun', 'Mar', 'Mié', 'Jue', 'Vie', 'Sáb'];
	const meses = ['ene', 'feb', 'mar', 'abr', 'may', 'jun', 'jul', 'ago', 'sep', 'oct', 'nov', 'dic'];
	return `${dias[date.getDay()]} ${date.getDate()} ${meses[date.getMonth()]}`;
}

export function formatFechaConDia(iso: string): string {
	if (!iso) return '—';
	const date = new Date(iso + 'T12:00:00');
	const dias = ['Domingo', 'Lunes', 'Martes', 'Miércoles', 'Jueves', 'Viernes', 'Sábado'];
	const meses = ['enero', 'febrero', 'marzo', 'abril', 'mayo', 'junio', 'julio', 'agosto', 'septiembre', 'octubre', 'noviembre', 'diciembre'];
	return `${dias[date.getDay()]} ${date.getDate()} de ${meses[date.getMonth()]} de ${date.getFullYear()}`;
}

export function formatMXN(val: number): string {
	if (!val) return '—';
	return new Intl.NumberFormat('es-MX', { style: 'currency', currency: 'MXN' }).format(val);
}

export interface StatusInfo {
	label: string;
	cls: string;
}

export function statusInfo(s: number): StatusInfo {
	switch (s) {
		case 0: return { label: 'Activo', cls: 'bg-green-bg text-green' };
		case 1: return { label: 'Cancelada', cls: 'bg-red-50 text-red-600' };
		case 2: return { label: 'Dev.Parcial', cls: 'bg-orange-50 text-orange-600' };
		case 3: return { label: 'Devolución', cls: 'bg-orange-50 text-orange-600' };
		case 4: return { label: 'Fact.Parcial', cls: 'bg-blue-50 text-blue-600' };
		case 5: return { label: 'Facturada', cls: 'bg-blue-50 text-blue-600' };
		case 6: return { label: 'Remit.Parcial', cls: 'bg-teal-50 text-teal-600' };
		case 7: return { label: 'Remitida', cls: 'bg-teal-50 text-teal-600' };
		default: return { label: 'Desconocido', cls: 'bg-slate-100 text-slate-400' };
	}
}

const tipoColorMap: Record<string, string> = {
	R: '#4a7fc1',
	N: '#e8820a',
	F: '#1f9254',
	Q: '#7c3aed',
	C: '#0891b2',
	O: '#2a5080',
};

export function tipoColor(t: string): string {
	return tipoColorMap[t] ?? '#64748b';
}

export function tipoLabel(t: string): string {
	const labels: Record<string, string> = {
		R: 'Remisión',
		N: 'Nota',
		F: 'Factura',
		Q: 'Cotización',
		C: 'Compra',
		O: 'Orden',
	};
	return labels[t] ?? t;
}

export function lineaImporte(cant: number, precio: number, pjedesc: number): number {
	return cant * precio * (1 - (pjedesc ?? 0) / 100);
}

/** Resta N días a una fecha ISO (YYYY-MM-DD) y devuelve la nueva fecha en el mismo formato. */
export function fechaMenos(iso: string, dias: number): string {
	const d = new Date(iso + 'T12:00:00');
	d.setDate(d.getDate() - dias);
	return d.toISOString().slice(0, 10);
}
