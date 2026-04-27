<script lang="ts">
	interface Props {
		message: string;
		onRetry?: () => void;
	}
	let { message, onRetry }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center py-12 px-4 text-center animate-fadeSlide">
	<div class="w-12 h-12 rounded-full bg-red-50 flex items-center justify-center mb-3">
		<svg class="w-6 h-6 text-red-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
			<circle cx="12" cy="12" r="10" />
			<line x1="12" y1="8" x2="12" y2="12" />
			<line x1="12" y1="16" x2="12.01" y2="16" />
		</svg>
	</div>
	<p class="text-[14px] font-medium text-slate-700 mb-1">Error al cargar datos</p>
	<p class="text-[12px] text-slate-400 mb-4 max-w-xs">{message}</p>
	{#if onRetry}
		<button
			onclick={onRetry}
			class="px-4 py-2 rounded-lg bg-navy text-white text-[13px] font-medium font-barlow
				hover:bg-navy-light active:bg-navy-dark transition-colors"
		>
			Reintentar
		</button>
	{/if}
</div>
