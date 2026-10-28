<script lang="ts">
	import { marked } from 'marked';
	import AluminumRibbing from '#lib/AluminumRibbing.svelte';
	import content from '#lib/content.md?raw';
	import flag from '#lib/assets/flag-orpheus-left.svg?raw';

	const html = marked
		.parse(content, { async: false })
		.replaceAll('<a ', '<a target="_blank" rel="noopener noreferrer" ');

	const tagline = 'Spend a weekend on a simple little emulator, earn a hackable handheld console!';
	const words = tagline.split(' ');
	const longest = Math.max(...words.map((word) => word.length));

	const url = 'https://chipper.hackclub.com/';
	const title = 'Chipper | Hack Club';
	const description =
		'Spend a weekend building a CHIP-8 emulator in any language and earn a free hackable handheld console from Hack Club. Open to teens 13 to 18.';
	const image = `${url}og.png`;
	const jsonLd = JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'WebSite',
		name: 'Chipper',
		url,
		description,
		publisher: { '@type': 'Organization', name: 'Hack Club', url: 'https://hackclub.com/' }
	}).replaceAll('<', '\\u003c');

	const rand = (max: number) => (Math.random() * 2 - 1) * max;
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<meta name="robots" content="index, follow, max-image-preview:large" />
	<link rel="canonical" href={url} />
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content="Chipper by Hack Club" />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={url} />
	<meta property="og:image" content={image} />
	<meta property="og:image:width" content="1200" />
	<meta property="og:image:height" content="630" />
	<meta property="og:image:alt" content="Chipper spelled out on blue keycaps" />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<meta name="twitter:image" content={image} />
	{@html `<script type="application/ld+json">${jsonLd}</script>`}
</svelte:head>

<main>
	<a class="flag" target="_blank" rel="noopener noreferrer" href="https://hackclub.com/" aria-label="Hack Club">
		{@html flag}
	</a>
	<header>
		<p class="org">Hack Club</p>
		<h1>
			<span class="sr-only">Chipper</span>
			{#each [...'Chipper'] as letter}
				<span class="key" aria-hidden="true" style:--dl={rand(0.025)} style:--dh={rand(3)}>
					<span>{letter}</span>
				</span>
			{/each}
		</h1>
		<p class="sr-only">{tagline}</p>
		<div class="tagline" aria-hidden="true" style:--span={longest * 1.12 - 0.12}>
			{#each words as word}
				<span class="word">
					{#each [...word] as letter}
						<span class="key" style:--dl={rand(0.025)} style:--dh={rand(3)}>
							<span>{letter}</span>
						</span>
					{/each}
				</span>
			{/each}
		</div>
	</header>

	<AluminumRibbing>
		<article>{@html html}</article>
		<div class="actions">
			<div class="console">
				<img src="/thumby-color.svg" alt="Line drawing of a hackable handheld console" />
			</div>
			<div class="cta">
				<p class="notice">All done with your CHIP-8 emulator?</p>
				<a class="key submit" target="_blank" rel="noopener noreferrer" href="/submit" style:--dl={rand(0.025)} style:--dh={rand(3)}>
					<span>Submit</span>
				</a>
			</div>
		</div>
		<footer>
			<a target="_blank" rel="noopener noreferrer" href="https://hackclub.com/">Hack Club</a>
			<a target="_blank" rel="noopener noreferrer" href="https://hackclub.com/privacy-and-terms">Terms &amp; Privacy Policy</a>
			<a target="_blank" rel="noopener noreferrer" href="https://forms.hackclub.com/bounty">Fulfillment bounty</a>
			<a target="_blank" rel="noopener noreferrer" href="https://security.hackclub.com/">Security</a>
		</footer>
	</AluminumRibbing>
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		min-height: 100vh;
	}

	footer {
		display: flex;
		flex-wrap: wrap;
		align-content: center;
		justify-content: center;
		gap: 0.5rem 1.5rem;
		margin-top: auto;
		margin-bottom: 0;
	}

	footer a {
		padding: 0.25rem 0;
		color: var(--blue-light);
	}

	.flag {
		position: absolute;
		top: 0;
		left: 10px;
		z-index: 999;
		width: 256px;
	}

	.flag :global(svg) {
		display: block;
		width: 100%;
		height: auto;
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: -1px;
		padding: 0;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
		border: 0;
	}

	article {
		margin-bottom: calc(2 * var(--cell) + 2px);
	}

	article,
	footer,
	.actions {
		--w: min(round(down, calc(100% - 5rem), var(--cell)), calc(42 * var(--cell)));
	}

	.actions {
		display: flex;
		align-items: center;
		width: calc(var(--w) - 2px);
		margin-left: calc(round(down, calc((100% - var(--w)) / 2), var(--cell)) + 2px);
		margin-bottom: calc(2 * var(--cell));
	}

	.console {
		flex: 0 1 auto;
		min-width: 0;
	}

	.console img {
		display: block;
		width: auto;
		max-width: 100%;
		height: calc(11 * var(--cell));
	}

	.cta {
		width: calc(min(20 * var(--cell), 100% + 2px) - 2px);
		margin-left: auto;
	}

	@container (max-width: 820px) {
		.actions {
			flex-direction: column;
			align-items: flex-start;
			margin-bottom: calc(3 * var(--cell));
		}

		.console {
			display: none;
		}

		.cta {
			margin-left: max(0px, round(down, calc((100% + 2px - 20 * var(--cell)) / 2), var(--cell)));
		}
	}

	.notice {
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		height: calc-size(auto, round(up, size + 2px, var(--cell)) - 2px);
		margin: 0 0 calc(var(--cell) + 2px);
		padding: 0 1rem;
		background: var(--black);
		font-size: 0.875rem;
		line-height: 1.6;
		text-align: center;
	}

	.key.submit {
		--k: calc(4 * var(--cell));
		--face: var(--beige);
		--edge: var(--beige-dark);
		--legend: var(--brown);
		width: calc(min(16 * var(--cell), 100% + 2px) - 2px);
		height: calc(4 * var(--cell));
		aspect-ratio: auto;
		margin-left: max(0px, round(down, calc((100% + 2px - 16 * var(--cell)) / 2), var(--cell)));
		margin-bottom: 0;
		text-decoration: none;
	}

	.key.submit span {
		letter-spacing: 0.1em;
	}

	article,
	footer {
		box-sizing: border-box;
		width: calc(var(--w) - 2px);
		height: calc-size(auto, round(up, size + 2px, var(--cell)) - 2px);
		margin-left: calc(round(down, calc((100% - var(--w)) / 2), var(--cell)) + 2px);
		padding: 2rem;
		background: var(--black);
		line-height: 1.6;
	}

	article > :global(:first-child) {
		margin-top: 0;
	}

	article > :global(:last-child) {
		margin-bottom: 0;
	}

	article :global(a) {
		color: var(--blue-light);
	}

	article :global(code) {
		padding: 0.1em 0.3em;
		background: var(--grey-800);
	}

	article :global(pre) {
		overflow-x: auto;
		padding: 1rem;
		background: var(--grey-800);
	}

	article :global(pre code) {
		padding: 0;
	}

	article :global(blockquote) {
		margin-left: 0;
		padding-left: 1rem;
		border-left: 2px solid var(--blue);
	}

	article :global(img) {
		max-width: 100%;
	}

	article :global(table) {
		border-collapse: collapse;
	}

	article :global(th),
	article :global(td) {
		padding: 0.3rem 0.8rem;
		border: 1px solid var(--blue);
	}

	header {
		box-sizing: border-box;
		width: 100%;
		padding: 2.5rem;
		container-type: inline-size;
		text-align: center;
	}

	@media (max-width: 1200px) {
		header {
			padding-top: calc(256px * 158 / 280 + 1rem);
		}
	}

	.org {
		margin: 0 0 0.5em;
		font-size: calc(min(80vw, 100cqw) / 7.72 * 0.6);
		line-height: 1;
	}

	h1 {
		position: relative;
		--k: calc(min(80vw, 100cqw) / 7.72);
		display: flex;
		justify-content: center;
		gap: calc(var(--k) * 0.12);
		margin: 0;
		line-height: 1;
	}

	.tagline {
		--k: min(48px, 100cqw / var(--span));
		--face: var(--beige);
		--edge: var(--beige-dark);
		--legend: var(--brown);
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: calc(var(--k) * 0.3) calc(var(--k) * 1.12);
		margin-top: max(1.5rem, 4cqw);
		line-height: 1;
	}

	.word {
		display: flex;
		gap: calc(var(--k) * 0.12);
	}

	.key {
		flex: none;
		width: 1em;
		aspect-ratio: 1;
		margin-bottom: 0.3em;
		display: grid;
		place-items: center;
		font-size: var(--k);
		box-sizing: border-box;
		--f: oklch(from var(--face, var(--blue)) calc(l + var(--dl)) c calc(h + var(--dh)));
		--e: oklch(from var(--edge, var(--blue-dark)) calc(l + var(--dl)) c calc(h + var(--dh)));
		border: 0.05em solid var(--e);
		border-radius: 0.03em;
		background: var(--f);
		box-shadow: 0 0.28em 0 var(--e);
		cursor: pointer;
		user-select: none;
	}

	.key:active {
		transform: translateY(0.2em);
		box-shadow: 0 0.08em 0 var(--e);
	}

	.key span {
		font-size: 0.55em;
		font-weight: 700;
		text-transform: uppercase;
		color: var(--legend, var(--white));
	}
</style>
