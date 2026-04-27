<script lang="ts">
	import { installUpdate, type Update } from '../lib/updater.js';

	interface Props {
		update: Update;
		onDismiss: () => void;
	}

	let { update, onDismiss }: Props = $props();

	let installing = $state(false);
	let downloaded = $state(0);
	let total = $state<number | null>(null);
	let error = $state<string | null>(null);

	function formatBytes(bytes: number): string {
		if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function progressPercent(): number {
		if (!total || total === 0) return 0;
		return Math.min(100, Math.round((downloaded / total) * 100));
	}

	async function install() {
		installing = true;
		error = null;
		try {
			await installUpdate(update, (dl, tot) => {
				downloaded = dl;
				total = tot;
			});
		} catch (e) {
			error = 'Error al instalar. Intenta de nuevo.';
			installing = false;
		}
	}
</script>

<div
	class="fixed bottom-4 right-4 z-50 w-80 rounded-xl shadow-2xl
		bg-[#0f1f38] border border-white/[0.12] overflow-hidden"
	role="alert"
>
	<div class="px-4 py-3 border-b border-white/[0.08]">
		<div class="flex items-center gap-2">
			<span class="w-2 h-2 rounded-full bg-green-400 shrink-0 animate-pulse"></span>
			<p class="text-[12px] font-semibold text-white tracking-wide">
				Nueva versión disponible
			</p>
		</div>
		<p class="text-[11px] text-white/50 mt-0.5 font-mono">
			v{update.version}
		</p>
	</div>

	<div class="px-4 py-3">
		{#if installing}
			<div class="mb-3">
				<div class="flex justify-between mb-1">
					<span class="text-[11px] text-white/50">
						{#if total}
							{formatBytes(downloaded)} / {formatBytes(total)}
						{:else}
							Descargando…
						{/if}
					</span>
					{#if total}
						<span class="text-[11px] text-white/50">{progressPercent()}%</span>
					{/if}
				</div>
				<div class="h-1.5 bg-white/10 rounded-full overflow-hidden">
					<div
						class="h-full bg-[#e11d48] rounded-full transition-all duration-300"
						style="width: {total ? progressPercent() : 40}%"
					></div>
				</div>
				<p class="text-[10px] text-white/35 mt-2">
					La aplicación se reiniciará al terminar.
				</p>
			</div>
		{:else}
			{#if update.body}
				<p class="text-[11px] text-white/60 mb-3 leading-relaxed line-clamp-3">
					{update.body}
				</p>
			{/if}

			{#if error}
				<p class="text-[11px] text-red-400 mb-2">{error}</p>
			{/if}

			<div class="flex gap-2">
				<button
					onclick={install}
					class="flex-1 py-1.5 rounded-lg bg-[#e11d48] text-white text-[12px] font-semibold
						hover:bg-[#c01840] active:scale-95 transition-all"
				>
					Instalar ahora
				</button>
				<button
					onclick={onDismiss}
					class="px-3 py-1.5 rounded-lg bg-white/[0.07] text-white/50 text-[12px]
						hover:bg-white/[0.12] hover:text-white/80 transition-all"
				>
					Después
				</button>
			</div>
		{/if}
	</div>
</div>
