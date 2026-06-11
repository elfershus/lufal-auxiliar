import './app.css';
import App from './App.svelte';
import { mount } from 'svelte';

function showFatalError(title: string, detail: string): void {
	document.body.innerHTML = `
		<div style="
			position:fixed;inset:0;display:flex;align-items:center;justify-content:center;
			background:#eceff4;font-family:sans-serif;padding:2rem;box-sizing:border-box;">
		  <div style="
			  max-width:640px;width:100%;background:#fff;border:1px solid #e2e8f0;
			  border-radius:12px;padding:2rem;box-shadow:0 4px 24px rgba(0,0,0,.08);">
			<div style="color:#dc2626;font-size:1rem;font-weight:700;margin-bottom:.5rem;">
			  Error al iniciar Lufal Auxiliar
			</div>
			<div style="color:#374151;font-size:.85rem;margin-bottom:1rem;word-break:break-word;">
			  ${title.replace(/</g, '&lt;')}
			</div>
			<pre style="
				background:#f8fafc;border:1px solid #e2e8f0;border-radius:8px;
				padding:1rem;font-size:.72rem;overflow:auto;max-height:260px;
				color:#4b5563;white-space:pre-wrap;word-break:break-all;margin:0;">
${detail.replace(/</g, '&lt;')}</pre>
			<div style="margin-top:.75rem;font-size:.72rem;color:#94a3b8;">
			  ${new Date().toLocaleString('es-MX')}
			</div>
		  </div>
		</div>`;
}

window.addEventListener('error', (e) => {
	showFatalError(e.message, e.error?.stack ?? `${e.filename}:${e.lineno}`);
});

window.addEventListener('unhandledrejection', (e) => {
	const reason = e.reason;
	const detail = reason instanceof Error
		? (reason.stack ?? reason.message)
		: String(reason);
	showFatalError('Promesa rechazada sin manejador', detail);
});

try {
	mount(App, { target: document.getElementById('app')! });
} catch (e) {
	showFatalError(
		'No se pudo montar la aplicación',
		e instanceof Error ? (e.stack ?? e.message) : String(e)
	);
}
