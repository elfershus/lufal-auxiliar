<script lang="ts">
	import type { Snippet } from 'svelte';
	import { auth } from '../lib/auth.svelte.js';

	interface Props {
		subtitle?: string;
		children: Snippet;
	}
	let {
		subtitle = 'Ingresa la contraseña para continuar',
		children
	}: Props = $props();

	let passwordInput = $state('');
	let passwordError = $state(false);
	let shake = $state(false);

	function submitPassword() {
		const ok = auth.unlock(passwordInput);
		passwordInput = '';
		if (ok) {
			passwordError = false;
		} else {
			passwordError = true;
			shake = true;
			setTimeout(() => (shake = false), 500);
		}
	}
</script>

{#if auth.unlocked}
	{@render children()}
{:else}
	<div class="min-h-[calc(100vh-8rem)] flex items-center justify-center px-4 py-6 md:px-6">
		<div class="w-full max-w-sm bg-surface rounded-card p-6 shadow-card animate-fadeSlide {shake ? 'animate-shake' : ''}">
			<div class="flex flex-col items-center mb-5">
				<div class="w-12 h-12 rounded-full bg-navy/10 flex items-center justify-center mb-3">
					<svg class="w-6 h-6 text-navy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
						<rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
						<path d="M7 11V7a5 5 0 0110 0v4" />
					</svg>
				</div>
				<h2 class="font-barlow-condensed text-[18px] font-bold text-navy">Acceso restringido</h2>
				<p class="text-[12px] text-slate-400 mt-1 text-center">{subtitle}</p>
			</div>

			<form onsubmit={(e) => { e.preventDefault(); submitPassword(); }}>
				<input
					type="password"
					placeholder="Contraseña"
					bind:value={passwordInput}
					class="w-full h-10 px-3 rounded-lg text-[14px] font-barlow mb-3
						bg-bg border {passwordError ? 'border-red-300' : 'border-slate-200'}
						focus:outline-none focus:ring-2 {passwordError ? 'focus:ring-red-300' : 'focus:ring-amber/60'}
						text-slate-700 placeholder:text-slate-400 transition-colors"
				/>
				{#if passwordError}
					<p class="text-[12px] text-red-500 mb-3 text-center">Contraseña incorrecta</p>
				{/if}
				<button
					type="submit"
					class="w-full h-10 rounded-lg bg-navy text-white text-[14px] font-medium font-barlow
						hover:bg-navy-light active:bg-navy-dark transition-colors"
				>
					Entrar
				</button>
			</form>
		</div>
	</div>
{/if}

<style>
	@keyframes shake {
		0%, 100% { transform: translateX(0); }
		20%       { transform: translateX(-8px); }
		40%       { transform: translateX(8px); }
		60%       { transform: translateX(-6px); }
		80%       { transform: translateX(6px); }
	}
	.animate-shake {
		animation: shake 0.45s ease-in-out;
	}
</style>
