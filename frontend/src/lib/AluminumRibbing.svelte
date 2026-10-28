<script lang="ts">
	import type { Snippet } from 'svelte';

	let { children }: { children?: Snippet } = $props();

	let section: HTMLElement;

	$effect(() => {
		const snap = () => {
			const cell = parseFloat(getComputedStyle(section).getPropertyValue('--cell'));
			const top = section.getBoundingClientRect().top + window.scrollY;
			section.style.minHeight = `${Math.ceil((window.innerHeight - top) / cell) * cell}px`;
		};
		const observer = new ResizeObserver(snap);
		observer.observe(document.body);
		if (section.previousElementSibling) observer.observe(section.previousElementSibling);
		window.addEventListener('resize', snap);
		snap();
		return () => {
			observer.disconnect();
			window.removeEventListener('resize', snap);
		};
	});
</script>

<div class="section" bind:this={section}>
	<div class="stage" aria-hidden="true">
		<div class="ribbing"></div>
	</div>
	<div class="content">
		{@render children?.()}
	</div>
</div>

<style>
	.section {
		--band: 14px;
		--angle: 6deg;
		--cell: 20px;
		--drop: calc(4 * var(--band) / cos(var(--angle)));
		position: relative;
		display: flex;
		flex: 1;
		flex-direction: column;
		container-type: inline-size;
	}

	.section::before {
		content: '';
		position: absolute;
		inset: 0;
		background-color: var(--blue-dark);
		background-image:
			linear-gradient(to right, var(--blue) 2px, transparent 2px),
			linear-gradient(to bottom, var(--blue) 2px, transparent 2px);
		background-size: var(--cell) var(--cell);
		clip-path: polygon(
			0 calc(var(--drop) - 1px),
			100% calc(100cqw * tan(var(--angle)) + var(--drop) - 1px),
			100% 100%,
			0 100%
		);
	}

	.content {
		position: relative;
		z-index: 1;
		display: flex;
		flex: 1;
		flex-direction: column;
		margin-top: calc(
			2 * var(--cell) - mod(calc(100cqw * tan(var(--angle)) + var(--drop)), var(--cell)) + 2px
		);
		margin-bottom: calc(3 * var(--cell));
	}

	.stage {
		position: relative;
		z-index: 1;
		overflow: hidden;
		height: calc(100cqw * tan(var(--angle)) + var(--drop));
	}

	.ribbing {
		position: absolute;
		top: 50%;
		left: -20%;
		width: 140%;
		height: calc(4 * var(--band));
		transform: translateY(-50%) rotate(var(--angle));
		background-image: linear-gradient(
			to bottom,
			var(--aluminum-dark) 0%,
			var(--aluminum) 40%,
			var(--aluminum-light) 55%,
			var(--aluminum-mid) 80%,
			var(--aluminum-dark) 100%
		);
		background-size: 100% var(--band);
	}
</style>
