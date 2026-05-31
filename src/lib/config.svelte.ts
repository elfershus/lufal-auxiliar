const NUMALM_KEY = 'lufal_numalm';
const NOMALM_KEY = 'lufal_nomalm';

function createAppConfig() {
	let numalm = $state(localStorage.getItem(NUMALM_KEY) ?? '');
	let nomalm = $state(localStorage.getItem(NOMALM_KEY) ?? '');

	return {
		get numalm() { return numalm; },
		set numalm(v: string) {
			numalm = v;
			localStorage.setItem(NUMALM_KEY, v);
		},
		get nomalm() { return nomalm; },
		set nomalm(v: string) {
			nomalm = v;
			localStorage.setItem(NOMALM_KEY, v);
		},
	};
}

export const appConfig = createAppConfig();

/** Al inicio de la app, sincroniza el almacén activo con default_numalm del config.toml.
 *  Siempre sobreescribe localStorage para limpiar valores obsoletos de versiones anteriores. */
export async function initNumalm(almacenes: { numalm: string; nomalm: string }[]) {
	const { getSucursalesConfig } = await import('./dbf.js');
	const cfg = await getSucursalesConfig();
	if (cfg.default_numalm) {
		appConfig.numalm = cfg.default_numalm;
		const alm = almacenes.find((a) => a.numalm === cfg.default_numalm);
		if (alm) appConfig.nomalm = alm.nomalm;
	} else {
		appConfig.numalm = '';
		appConfig.nomalm = '';
	}
}
