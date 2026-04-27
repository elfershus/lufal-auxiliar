<script lang="ts">
	interface Props {
		count?: number;
	}
	let { count = 5 }: Props = $props();
</script>

<div class="flex flex-col gap-3">
	{#each { length: count } as _, i (i)}
		<div class="bg-surface rounded-card p-4 shadow-card animate-fadeSlide" style="animation-delay: {i * 40}ms">
			<div class="flex items-start justify-between mb-3">
				<div class="h-5 w-16 rounded-full animate-shimmer"
					style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
				</div>
				<div class="h-5 w-20 rounded-full animate-shimmer"
					style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
				</div>
			</div>
			<div class="h-4 w-3/4 rounded animate-shimmer mb-2"
				style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
			</div>
			<div class="h-3 w-1/2 rounded animate-shimmer"
				style="background: linear-gradient(90deg, #e2e8f0 25%, #f0f4f8 50%, #e2e8f0 75%); background-size: 400% 100%">
			</div>
		</div>
	{/each}
</div>
