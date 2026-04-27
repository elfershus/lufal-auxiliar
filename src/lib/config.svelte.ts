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
