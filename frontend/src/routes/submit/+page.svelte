<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';

	type Me = { signed_in: boolean; first_name: string; hackatime_linked: boolean };
	type Project = { name: string; seconds: number };

	const MAX_SCREENSHOT_BYTES = 5_000_000;
	const HOUR = 3600;
	const SCREENSHOT_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];
	const DRAFT_KEY = 'chipper-submit-draft';

	let me = $state<Me | null>(null);
	let meError = $state('');
	let projects = $state<Project[] | null>(null);
	let projectsError = $state('');
	const ticked = new SvelteSet<string>();
	let search = $state('');

	let repositoryUrl = $state('');
	let codeUrl = $state('');
	let description = $state('');
	let screenshot = $state<FileList | undefined>();
	let screenshotError = $state('');
	let grant = $state(false);

	let submitting = $state(false);
	let submitError = $state('');
	let submitted = $state(false);
	let restored = false;

	let shown = $derived(
		(projects ?? []).filter((p) => p.name.toLowerCase().includes(search.trim().toLowerCase()))
	);
	let total = $derived(
		(projects ?? []).filter((p) => ticked.has(p.name)).reduce((sum, p) => sum + p.seconds, 0)
	);
	let tier = $derived(
		total >= 30 * HOUR
			? "You've unlocked both devices, the Thumby Color and the LILYGO T-Deck Max. Well done!"
			: total >= 25 * HOUR
				? "You've reached enough time for the LILYGO T-Deck Max! (make it to 30 hours to get both devices)"
				: total >= 10 * HOUR
					? "You've reached enough time for the Thumby Color!"
					: 'Not quite enough time tracked yet, make sure you reach 10 hours'
	);
	let canSubmit = $derived(
		!!me?.signed_in && me.hackatime_linked && total >= 10 * HOUR && !submitting
	);

	$effect(() => {
		const draft = JSON.stringify({ repositoryUrl, codeUrl, description, grant, projects: [...ticked] });
		if (!restored) return;
		try {
			localStorage.setItem(DRAFT_KEY, draft);
		} catch {}
	});

	function draftStore(mode: IDBTransactionMode) {
		return new Promise<IDBObjectStore>((resolve, reject) => {
			const req = indexedDB.open('chipper', 1);
			req.onupgradeneeded = () => req.result.createObjectStore('draft');
			req.onsuccess = () => resolve(req.result.transaction('draft', mode).objectStore('draft'));
			req.onerror = () => reject(req.error);
		});
	}

	async function saveScreenshot(file: File | undefined) {
		try {
			const store = await draftStore('readwrite');
			if (file) store.put(file, 'screenshot');
			else store.delete('screenshot');
		} catch {}
	}

	async function restoreDraft() {
		try {
			const draft = JSON.parse(localStorage.getItem(DRAFT_KEY) ?? '{}');
			repositoryUrl = draft.repositoryUrl ?? '';
			codeUrl = draft.codeUrl ?? '';
			description = draft.description ?? '';
			grant = draft.grant === true;
			for (const name of draft.projects ?? []) ticked.add(name);
		} catch {}
		restored = true;

		try {
			const store = await draftStore('readonly');
			const file = await new Promise<File | undefined>((resolve, reject) => {
				const req = store.get('screenshot');
				req.onsuccess = () => resolve(req.result);
				req.onerror = () => reject(req.error);
			});
			if (file && SCREENSHOT_TYPES.includes(file.type) && !screenshot?.length) {
				const transfer = new DataTransfer();
				transfer.items.add(file);
				screenshot = transfer.files;
			}
		} catch {}
	}

	function duration(seconds: number) {
		return `${Math.floor(seconds / HOUR)}h ${Math.floor((seconds % HOUR) / 60)}m`;
	}

	async function errorMessage(res: Response) {
		try {
			return (await res.json()).error ?? `Something went wrong (${res.status})`;
		} catch {
			return `Something went wrong (${res.status})`;
		}
	}

	async function loadMe() {
		try {
			const res = await fetch('/api/me');
			if (res.ok) {
				me = await res.json();
				meError = '';
			} else meError = await errorMessage(res);
		} catch {
			meError = "Couldn't reach the server, please refresh";
		}
	}

	async function signOut() {
		try {
			const res = await fetch('/api/logout', { method: 'POST' });
			if (!res.ok) throw new Error();
			projects = null;
			await loadMe();
		} catch {
			meError = "Couldn't sign out, please refresh and try again";
		}
	}

	async function loadProjects() {
		try {
			const res = await fetch('/api/hackatime/projects');
			if (res.ok) {
				projects = await res.json();
				for (const name of ticked) if (!projects?.some((p) => p.name === name)) ticked.delete(name);
			} else if (res.status === 401 && me) me.hackatime_linked = false;
			else projectsError = await errorMessage(res);
		} catch {
			projectsError = "Couldn't load your Hackatime projects, please refresh";
		}
	}

	function checkScreenshot(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		screenshotError = '';
		if (file && !SCREENSHOT_TYPES.includes(file.type))
			screenshotError = 'Screenshot must be a PNG, JPEG, GIF or WebP image';
		else if (file && file.size > MAX_SCREENSHOT_BYTES)
			screenshotError = 'Screenshot must be 5 MB or smaller';
		if (screenshotError) {
			input.value = '';
			screenshot = undefined;
		}
		saveScreenshot(screenshotError ? undefined : file);
	}

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		const file = screenshot?.[0];
		if (!canSubmit || !file) return;

		const body = new FormData();
		body.append('repository_url', repositoryUrl);
		body.append('code_url', codeUrl);
		body.append('description', description);
		body.append('screenshot', file);
		for (const name of ticked) body.append('projects', name);
		body.append('grant', String(grant));

		submitting = true;
		submitError = '';
		try {
			const res = await fetch('/api/submit', { method: 'POST', body });
			if (res.ok) {
				submitted = true;
				try {
					localStorage.removeItem(DRAFT_KEY);
				} catch {}
				saveScreenshot(undefined);
			} else {
				submitError = await errorMessage(res);
				if (res.status === 401) await loadMe();
			}
		} catch {
			submitError = "Couldn't reach the server, please try again";
		} finally {
			submitting = false;
		}
	}

	onMount(async () => {
		restoreDraft();
		await loadMe();
		if (me?.hackatime_linked) await loadProjects();
	});
</script>

<svelte:head>
	<title>Submit | Chipper | Hack Club</title>
	<meta
		name="description"
		content="Submit your CHIP-8 emulator to Chipper and earn a hackable handheld console from Hack Club."
	/>
</svelte:head>

<div class="grid">
	<a class="back" href="/" aria-label="Back">
		<svg viewBox="0 0 24 24" aria-hidden="true">
			<path d="M15 5 8 12l7 7" />
		</svg>
	</a>
	<main>
		<h1>Submit your CHIP-8 emulator!</h1>

		{#if submitted}
			<p role="status">Submitted! Thanks for building a CHIP-8 emulator.</p>
		{:else}
			<div class="account">
				{#if meError}
					<p class="error" role="alert">{meError}</p>
				{:else if me?.signed_in}
					<p>Signed in as {me.first_name}</p>
					<button type="button" class="link" onclick={signOut}>Sign out</button>
				{:else if me}
					<a class="btn" href="/api/login">Sign in with Hack Club Auth</a>
				{/if}
			</div>

			<form aria-label="Submit your CHIP-8 emulator" onsubmit={submit}>
				<fieldset disabled={!me?.signed_in}>
					<div class="field">
						<label for="repository-url">Repository URL</label>
						<p class="hint" id="repository-url-hint">
							Link to the code on GitHub, GitLab, Bitbucket, etc. Must have a reasonably detailed README.md
						</p>
						<input
							id="repository-url"
							type="url"
							required
							aria-describedby="repository-url-hint"
							bind:value={repositoryUrl}
						/>
					</div>

					<div class="field">
						<label for="code-url">Code URL</label>
						<p class="hint" id="code-url-hint">
							Link to spot where your project can be downloaded. This should be something like a GitHub Release
						</p>
						<input id="code-url" type="url" required aria-describedby="code-url-hint" bind:value={codeUrl} />
					</div>

					<div class="field">
						<label for="screenshot">Screenshot</label>
						<p class="hint" id="screenshot-hint">
							Screenshot of your project in action. Must be of the project actually running, not the code
						</p>
						<input
							id="screenshot"
							type="file"
							accept={SCREENSHOT_TYPES.join(',')}
							required
							aria-describedby="screenshot-hint"
							bind:files={screenshot}
							onchange={checkScreenshot}
						/>
						{#if screenshotError}
							<p class="error" role="alert">{screenshotError}</p>
						{/if}
					</div>

					<div class="field">
						<label for="description">Description</label>
						<p class="hint" id="description-hint">Quick description of what your project is</p>
						<textarea
							id="description"
							rows="4"
							maxlength="10000"
							required
							aria-describedby="description-hint"
							bind:value={description}
						></textarea>
					</div>

					<div class="field" role="group" aria-labelledby="hackatime-label" aria-describedby="hackatime-hint">
						<span class="label" id="hackatime-label">Hackatime</span>
						<p class="hint" id="hackatime-hint">Tick off the Hackatime-tracked projects for your emulator</p>
						{#if !me?.hackatime_linked}
							<button type="button" class="btn" onclick={() => (location.href = '/api/hackatime/login')}>
								Link Hackatime account
							</button>
						{:else if projectsError}
							<p class="error" role="alert">{projectsError}</p>
						{:else if !projects}
							<p>Loading projects</p>
						{:else}
							<ul class="projects">
								{#each shown as project (project.name)}
									<li>
										<label>
											<input
												type="checkbox"
												checked={ticked.has(project.name)}
												onchange={(e) =>
													e.currentTarget.checked ? ticked.add(project.name) : ticked.delete(project.name)}
											/>
											<span class="name">{project.name}</span>
											<span class="time">{duration(project.seconds)}</span>
										</label>
									</li>
								{:else}
									<li class="empty">
										{projects.length ? 'No matching projects' : 'No Hackatime projects yet'}
									</li>
								{/each}
							</ul>
							<input type="search" placeholder="Search projects" aria-label="Search projects" bind:value={search} />
							<p class="total" aria-live="polite">{duration(total)} tracked</p>
							{#if ticked.size}
								<p class="tier" class:short={total < 10 * HOUR} aria-live="polite">{tier}</p>
							{/if}
						{/if}
					</div>

					{#if submitError}
						<p class="error" role="alert">{submitError}</p>
					{/if}

					<p class="hint">
						By submitting, I understand that Hack Club is not liable for any customs fees that I may be charged in
						order to receive the prize
					</p>
					<label class="grant">
						<input type="checkbox" bind:checked={grant} />
						Send me a grant instead so I can buy the prize from a spot with lower customs fees
					</label>

					<button type="submit" class="btn submit" disabled={!canSubmit}>
						{submitting ? 'Submitting' : 'Submit'}
					</button>
				</fieldset>
			</form>
		{/if}
	</main>
</div>

<style>
	.grid {
		position: relative;
		display: flow-root;
		--cell: 20px;
		--w: min(round(down, calc(100% - 5rem), var(--cell)), calc(30 * var(--cell)));
		box-sizing: border-box;
		min-height: 100vh;
		padding-bottom: calc(4 * var(--cell));
		background-color: var(--blue-dark);
		background-image:
			linear-gradient(to right, var(--blue) 2px, transparent 2px),
			linear-gradient(to bottom, var(--blue) 2px, transparent 2px);
		background-size: var(--cell) var(--cell);
	}

	main {
		box-sizing: border-box;
		width: calc(var(--w) - 2px);
		height: calc-size(auto, round(up, size + 2px, var(--cell)) - 2px);
		margin-top: calc(4 * var(--cell) + 2px);
		margin-left: calc(round(down, calc((100% - var(--w)) / 2), var(--cell)) + 2px);
		padding: 2rem;
		background: var(--black);
		line-height: 1.6;
	}

	h1 {
		margin: 0;
		font-size: 1.5rem;
	}

	.account {
		margin: 1.5rem 0;
	}

	.account p {
		margin: 0;
	}

	.link {
		padding: 0;
		border: 0;
		background: none;
		color: var(--blue-light);
		font: inherit;
		font-size: 0.875rem;
		text-decoration: underline;
		cursor: pointer;
	}

	fieldset {
		margin: 0;
		padding: 0;
		border: 0;
		min-width: 0;
	}

	fieldset:disabled {
		opacity: 0.45;
	}

	.field {
		margin-bottom: 1.75rem;
	}

	label,
	.label {
		display: block;
		font-weight: 700;
	}

	.hint {
		margin: 0.1rem 0 0.6rem;
		font-size: 0.875rem;
		color: var(--grey-400);
	}

	input:not([type='checkbox']),
	textarea {
		display: block;
		box-sizing: border-box;
		width: 100%;
		padding: 0.55rem 0.7rem;
		border: 1px solid var(--grey-700);
		border-radius: 0;
		background: var(--grey-900);
		color: var(--text);
		font: inherit;
		font-size: 0.9rem;
		transition: border-color 0.1s;
	}

	input:not([type='checkbox']):hover,
	textarea:hover {
		border-color: var(--grey-600);
	}

	input:not([type='checkbox']):focus,
	textarea:focus {
		border-color: var(--beige);
		outline: none;
	}

	input::placeholder {
		color: var(--grey-500);
	}

	textarea {
		resize: vertical;
	}

	input[type='file'] {
		padding: 0.35rem;
		color: var(--grey-400);
		cursor: pointer;
	}

	input[type='file']::file-selector-button {
		margin-right: 0.75rem;
		padding: 0.25rem 0.7rem;
		border: 1px solid var(--grey-700);
		background: var(--grey-800);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	input[type='file']:hover::file-selector-button {
		border-color: var(--grey-600);
	}

	input[type='checkbox'] {
		flex: none;
		width: 1.1rem;
		height: 1.1rem;
		margin: 0;
		accent-color: var(--beige);
	}

	.grant {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
		font-size: 0.875rem;
		font-weight: 400;
		cursor: pointer;
	}

	.projects {
		max-height: 16rem;
		overflow-y: auto;
		margin: 0 0 0.5rem;
		padding: 0;
		border: 1px solid var(--grey-700);
		background: var(--grey-900);
		list-style: none;
		font-size: 0.9rem;
	}

	.projects li + li {
		border-top: 1px solid var(--grey-800);
	}

	.projects label {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem 0.75rem;
		font-weight: 400;
		cursor: pointer;
	}

	.projects label:hover {
		background: var(--grey-800);
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.time {
		flex: none;
		color: var(--grey-400);
	}

	.empty {
		padding: 0.5rem 0.75rem;
		color: var(--grey-400);
	}

	.total {
		margin: 0.75rem 0 0;
		font-weight: 700;
	}

	.tier {
		margin: 0.25rem 0 0;
		color: var(--beige);
	}

	.tier.short {
		color: var(--red);
	}

	.error {
		margin: 0.5rem 0;
		color: var(--red);
	}

	.btn {
		display: inline-block;
		margin-bottom: 6px;
		padding: 0.6rem 1.1rem;
		border: 2px solid var(--beige-dark);
		border-radius: 0;
		background: var(--beige);
		box-shadow: 0 6px 0 var(--beige-dark);
		color: var(--brown);
		font: inherit;
		font-weight: 700;
		text-decoration: none;
		cursor: pointer;
	}

	.btn:active:not(:disabled) {
		transform: translateY(4px);
		box-shadow: 0 2px 0 var(--beige-dark);
	}

	.btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	fieldset:disabled .btn:disabled {
		opacity: 1;
	}

	.submit {
		width: 100%;
	}

	.back {
		position: absolute;
		top: calc(var(--cell) + 2px);
		left: calc(var(--cell) + 2px);
		display: grid;
		place-items: center;
		width: calc(2 * var(--cell) - 2px);
		height: calc(2 * var(--cell) - 2px);
		background: var(--black);
		color: var(--white);
	}

	.back svg {
		width: 60%;
		height: 60%;
		fill: none;
		stroke: currentColor;
		stroke-width: 2;
		stroke-linecap: square;
		stroke-linejoin: miter;
	}
</style>
