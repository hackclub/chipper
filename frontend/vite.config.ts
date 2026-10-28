import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			adapter: adapter({ fallback: '200.html' }),

			prerender: {
				handleHttpError: ({ path, message }) => {
					if (path === '/submit') return;
					throw new Error(message);
				}
			}
		})
	],
	server: {
		proxy: {
			'/api': 'http://127.0.0.1:8080'
		}
	}
});
