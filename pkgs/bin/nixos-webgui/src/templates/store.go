package templates

import (
	"time"

	g "maragu.dev/gomponents"
	"maragu.dev/gomponents/html"

	"github.com/krezh/nixos-webgui/system"
)

// Store renders the Nix store management page.
func Store(storeStats *system.StoreStats, cachedAt time.Time) g.Node {
	return Base("Nix Store", StoreContent(storeStats, cachedAt))
}

// StoreContent renders the store management content with statistics and actions.
func StoreContent(storeStats *system.StoreStats, cachedAt time.Time) g.Node {
	return html.Div(
		html.Class("space-y-6"),
		// Store Statistics
		html.Div(
			html.Class("card"),
			html.Div(
				html.Class("flex justify-between items-center mb-4"),
				CardHeading("Store Statistics"),
				html.Div(
					html.Class("tooltip"),
					html.Button(
						g.Attr("onclick", "loadStoreStats(true)"),
						html.ID("refresh-store-btn"),
						html.Class("btn btn-sm btn-secondary"),
						g.Raw(`<svg class="w-4 h-4 inline-block mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
						</svg>`),
						html.Span(
							html.ID("refresh-store-text"),
							g.Text("Refresh"),
						),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("du -sb /nix/store")),
						html.Br(),
						html.Code(g.Text("nix-store --gc --print-dead")),
					),
				),
			),
			html.Div(
				html.Class("grid grid-cols-1 md:grid-cols-3 gap-4 mb-4"),
				html.Div(
					html.Class("text-center p-4 bg-gray-50 dark:bg-gray-800 rounded-lg"),
					html.Div(
						html.Class("text-sm text-gray-600 dark:text-gray-400"),
						g.Text("Total Store Size"),
					),
					html.Div(
						html.ID("store-total"),
						html.Class("text-2xl font-bold mt-1 h-8 flex items-center justify-center"),
						func() g.Node {
							if storeStats != nil {
								return g.Text(system.FormatBytes(storeStats.TotalSize))
							}
							return g.Text("-")
						}(),
					),
				),
				html.Div(
					html.Class("text-center p-4 bg-gray-50 dark:bg-gray-800 rounded-lg"),
					html.Div(
						html.Class("text-sm text-gray-600 dark:text-gray-400"),
						g.Text("In Use"),
					),
					html.Div(
						html.ID("store-alive"),
						html.Class("text-2xl font-bold mt-1 text-green-600 h-8 flex items-center justify-center"),
						func() g.Node {
							if storeStats != nil {
								return g.Text(system.FormatBytes(storeStats.AliveSize))
							}
							return g.Text("-")
						}(),
					),
				),
				html.Div(
					html.Class("text-center p-4 bg-gray-50 dark:bg-gray-800 rounded-lg"),
					html.Div(
						html.Class("text-sm text-gray-600 dark:text-gray-400"),
						g.Text("Reclaimable"),
					),
					html.Div(
						html.ID("store-dead"),
						html.Class("text-2xl font-bold mt-1 text-orange-600 h-8 flex items-center justify-center"),
						func() g.Node {
							if storeStats != nil {
								return g.Text(system.FormatBytes(storeStats.DeadSize))
							}
							return g.Text("-")
						}(),
					),
				),
			),
			func() g.Node {
				if storeStats != nil {
					return html.Div(
						html.ID("store-cache-info"),
						html.Class("text-xs text-gray-500 dark:text-gray-400 mb-4"),
						g.Textf("Cached — last updated %s", cachedAt.Format("02/01/2006, 15:04:05")),
					)
				}
				return html.Div(
					html.ID("store-cache-info"),
					html.Class("text-xs text-gray-500 dark:text-gray-400 mb-4 hidden"),
				)
			}(),
		),
		// Store Management Actions
		html.Div(
			html.Class("card"),
			CardHeading("Store Management"),
			html.P(
				html.Class("text-sm text-gray-600 dark:text-gray-400 mb-4"),
				g.Text("Manage your Nix store. You can run garbage collection to remove unreachable paths or optimize the store to deduplicate files."),
			),
			html.Div(
				html.Class("flex gap-4"),
				html.Div(
					html.Class("tooltip"),
					html.Button(
						g.Attr("onclick", "showDialog('gc-dialog')"),
						html.Class("btn btn-primary"),
						g.Raw(`<svg class="w-5 h-5 inline-block mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
						</svg>`),
						g.Text("Run Garbage Collection"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("sudo nix-collect-garbage -d")),
					),
				),
				html.Div(
					html.Class("tooltip"),
					html.Button(
						g.Attr("onclick", "showDialog('optimize-dialog')"),
						html.Class("btn btn-secondary"),
						g.Raw(`<svg class="w-5 h-5 inline-block mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
						</svg>`),
						g.Text("Optimize Store"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("sudo nix-store --optimise")),
					),
				),
			),
		),
		// GC Dialog
		html.Div(
			html.ID("gc-dialog"),
			html.Class("hidden fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"),
			html.Div(
				html.Class("relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white dark:bg-gray-800"),
				html.Div(
					html.Class("mt-3"),
					html.H3(
						html.Class("text-lg font-medium leading-6 mb-4"),
						g.Text("Run Garbage Collection"),
					),
					html.Div(
						html.Class("mt-2 space-y-4"),
						html.P(
							html.Class("text-sm text-gray-600 dark:text-gray-400"),
							g.Text("This will delete all store paths that are not reachable from any generation. This can free up significant disk space."),
						),
						PasswordInput("gc-password"),
					),
					html.Div(
						html.Class("flex gap-3 mt-6"),
						html.Button(
							g.Attr("onclick", "confirmGC()"),
							html.Class("btn btn-primary flex-1"),
							g.Text("Start"),
						),
						html.Button(
							g.Attr("onclick", "hideGCDialog()"),
							html.Class("btn btn-secondary flex-1"),
							g.Text("Cancel"),
						),
					),
				),
			),
		),
		// Optimize Dialog
		html.Div(
			html.ID("optimize-dialog"),
			html.Class("hidden fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"),
			html.Div(
				html.Class("relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white dark:bg-gray-800"),
				html.Div(
					html.Class("mt-3"),
					html.H3(
						html.Class("text-lg font-medium leading-6 mb-4"),
						g.Text("Optimize Store"),
					),
					html.Div(
						html.Class("mt-2 space-y-4"),
						html.P(
							html.Class("text-sm text-gray-600 dark:text-gray-400"),
							g.Text("This will deduplicate identical files in the store using hard links. This can take a while but saves disk space."),
						),
						PasswordInput("optimize-password"),
					),
					html.Div(
						html.Class("flex gap-3 mt-6"),
						html.Button(
							g.Attr("onclick", "confirmOptimize()"),
							html.Class("btn btn-primary flex-1"),
							g.Text("Start"),
						),
						html.Button(
							g.Attr("onclick", "hideOptimizeDialog()"),
							html.Class("btn btn-secondary flex-1"),
							g.Text("Cancel"),
						),
					),
				),
			),
		),
		html.Script(
			g.Raw(`
		// Wire up Enter key for password fields
		document.addEventListener('DOMContentLoaded', function() {
			var gcp = document.getElementById('gc-password');
			if (gcp) gcp.addEventListener('keypress', function(e) { if (e.key === 'Enter') confirmGC(); });
			var op = document.getElementById('optimize-password');
			if (op) op.addEventListener('keypress', function(e) { if (e.key === 'Enter') confirmOptimize(); });
		});

		window.addEventListener('load', function() {
			const storeTotalText = document.getElementById('store-total').textContent.trim();
			if (storeTotalText === '-') {
				loadStoreStats(false);
			}
		});

		function loadStoreStats(refresh) {
			const btn = document.getElementById('refresh-store-btn');
			const btnText = document.getElementById('refresh-store-text');
			const originalText = btnText.textContent;
			const totalEl = document.getElementById('store-total');
			const aliveEl = document.getElementById('store-alive');
			const deadEl = document.getElementById('store-dead');

			const willFetchNewData = refresh || totalEl.textContent.trim() === '-';
			if (willFetchNewData) {
				const spinnerHTML = makeSpinner();
				totalEl.innerHTML = spinnerHTML;
				aliveEl.innerHTML = spinnerHTML;
				deadEl.innerHTML = spinnerHTML;
			}

			btn.disabled = true;
			btnText.textContent = 'Loading...';

			fetch(refresh ? '/api/store/stats?refresh=true' : '/api/store/stats')
				.then(response => {
					if (!response.ok) throw new Error('Failed to fetch store stats');
					return response.json();
				})
				.then(data => {
					totalEl.textContent = formatBytes(data.total_size);
					aliveEl.textContent = formatBytes(data.alive_size);
					deadEl.textContent = formatBytes(data.dead_size);

					if (data.cached_at) {
						const cacheInfo = document.getElementById('store-cache-info');
						cacheInfo.classList.remove('hidden');
						const ts = new Date(data.cached_at).toLocaleString('en-GB', { hour12: false });
						cacheInfo.textContent = data.cached
							? 'Cached \u2014 last updated ' + ts
							: 'Data updated at ' + ts;
					}

					btn.disabled = false;
					btnText.textContent = 'Refresh';
				})
				.catch(err => {
					console.error('Failed to load store stats:', err);
					totalEl.textContent = 'Error';
					aliveEl.textContent = 'Error';
					deadEl.textContent = 'Error';
					btn.disabled = false;
					btnText.textContent = originalText;
				});
		}

		function formatBytes(bytes) {
			const unit = 1024;
			if (bytes < unit) return bytes + ' B';
			let div = unit, exp = 0, n = Math.floor(bytes / unit);
			while (n >= unit) { div *= unit; exp++; n = Math.floor(n / unit); }
			return (bytes / div).toFixed(1) + ' ' + ['K', 'M', 'G', 'T', 'P', 'E'][exp] + 'iB';
		}

		function hideGCDialog() {
			hideDialog('gc-dialog');
			document.getElementById('gc-password').value = '';
		}

		function confirmGC() {
			const password = document.getElementById('gc-password').value;
			if (!password) { alert('Please enter your sudo password'); return; }

			fetch('/api/store/gc', {
				method: 'POST',
				headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
				body: 'password=' + encodeURIComponent(password)
			})
			.then(response => response.text())
			.then(message => {
				alert(message);
				hideGCDialog();
				loadStoreStats(true);
			})
			.catch(err => {
				alert('Garbage collection failed: ' + err.message);
				hideGCDialog();
			});
		}

		function hideOptimizeDialog() {
			hideDialog('optimize-dialog');
			document.getElementById('optimize-password').value = '';
		}

		function confirmOptimize() {
			const password = document.getElementById('optimize-password').value;
			if (!password) { alert('Please enter your sudo password'); return; }

			fetch('/api/store/optimize', {
				method: 'POST',
				headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
				body: 'password=' + encodeURIComponent(password)
			})
			.then(response => response.text())
			.then(message => {
				alert(message);
				hideOptimizeDialog();
				loadStoreStats(true);
			})
			.catch(err => {
				alert('Store optimization failed: ' + err.message);
				hideOptimizeDialog();
			});
		}
			`),
		),
	)
}
