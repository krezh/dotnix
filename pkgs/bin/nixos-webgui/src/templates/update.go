package templates

import (
	g "maragu.dev/gomponents"
	"maragu.dev/gomponents/html"
)

// Update renders the update page.
func Update() g.Node {
	return Base("Update", UpdateContent())
}

// UpdateContent renders the update page content with buttons and output area.
func UpdateContent() g.Node {
	return html.Div(
		html.Class("space-y-6"),
		html.Div(
			html.Class("card"),
			html.Div(
				html.Class("flex items-center justify-between mb-4"),
				CardHeading("System Update"),
				html.Div(
					html.Class("flex items-center gap-2"),
					html.Div(
						html.ID("git-status-indicator"),
						HxGet("/api/update/git-status"),
						HxTrigger("load"),
						HxSwap("outerHTML"),
					),
					html.Div(
						html.Class("tooltip"),
						html.Button(
							HxPost("/api/update/git-status/refresh"),
							HxTarget("#git-status-indicator"),
							HxSwap("outerHTML"),
							html.Class("p-1.5 rounded-md text-gray-400 hover:text-gray-200 hover:bg-gray-700 transition-colors"),
							g.Raw(`<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
							</svg>`),
						),
						html.Span(
							html.Class("tooltip-text"),
							g.Text("Check for updates"),
						),
					),
				),
			),
			html.P(
				html.Class("text-gray-600 dark:text-gray-400 mb-4"),
				g.Text("This will pull the latest changes from your NixOS configuration repository and rebuild the system using "),
				html.Code(
					html.Class("bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded"),
					g.Text("nixos-rebuild switch"),
				),
				g.Text("."),
			),
			html.Div(
				html.Class("flex gap-4"),
				html.Div(
					html.Class("tooltip"),
					html.Button(
						HxPost("/api/update/start"),
						HxTarget("#update-output"),
						HxSwap("innerHTML"),
						g.Attr("hx-disabled-elt", "this"),
						html.Class("btn btn-primary"),
						g.Raw(`<svg class="w-5 h-5 inline-block mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
						</svg>`),
						g.Text("Start Update"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("sudo nixos-rebuild build")),
					),
				),
				html.Div(
					html.Class("tooltip"),
					html.Button(
						HxPost("/api/update/flake"),
						HxTarget("#update-output"),
						HxSwap("innerHTML"),
						g.Attr("hx-disabled-elt", "this"),
						html.Class("btn btn-secondary"),
						g.Raw(`<svg class="w-5 h-5 inline-block mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
						</svg>`),
						g.Text("Update Flake Inputs"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("nix flake update")),
					),
				),
			),
		),
		html.Div(
			html.ID("update-output"),
			html.Class("card"),
			html.H3(
				html.Class("text-lg font-semibold mb-4"),
				g.Text("Output"),
			),
			html.Div(
				html.Class("text-gray-500 dark:text-gray-400 italic"),
				g.Text(`Click "Start Update" to begin the update process. Logs will appear here.`),
			),
		),
	)
}

// UpdateProgressInitial renders the initial progress view with log streaming.
func UpdateProgressInitial(logs []string) g.Node {
	return html.Div(g.Group([]g.Node{
		html.Div(
			html.ID("update-container"),
			html.Div(
				html.Class("flex items-center gap-3 mb-4"),
				html.H3(
					html.Class("text-lg font-semibold"),
					g.Text("Output"),
				),
				html.Div(
					html.ID("spinner-container"),
					html.Class("flex items-center gap-2"),
					html.Div(
						html.Class("spinner-7"),
						html.Div(),
						html.Div(),
						html.Div(),
					),
					html.Span(
						html.Class("text-sm font-medium"),
						g.Text("Update in progress..."),
					),
				),
			),
			html.Pre(
				html.Class("log-output"),
				html.ID("log-content"),
			),
			html.Div(html.ID("status-area")),
		),
		html.Script(
			g.Raw(`
		(function() {
			const logContent = document.getElementById('log-content');
			const statusArea = document.getElementById('status-area');
			const spinnerContainer = document.getElementById('spinner-container');
			let isUserScrolling = false;
			let scrollTimeout = null;

			// Detect when user manually scrolls
			logContent.addEventListener('scroll', function() {
				const isAtBottom = logContent.scrollHeight - logContent.clientHeight <= logContent.scrollTop + 50;
				isUserScrolling = !isAtBottom;

				// Reset user scrolling flag after 2 seconds of no scroll
				clearTimeout(scrollTimeout);
				scrollTimeout = setTimeout(() => {
					isUserScrolling = false;
				}, 2000);
			});

		// Connect to SSE stream
		console.log('Connecting to SSE stream...');
		const eventSource = new EventSource('/api/update/stream');

		eventSource.onopen = function() {
			console.log('SSE connection opened');
		};

		eventSource.onmessage = function(event) {
			const line = event.data;
			console.log('Received log line:', line);

			if (line === '__COMPLETE__' || line === '__APPLY_COMPLETE__') {
				// These are handled by the complete event listener
				return;
			}

			if (line) {
				logContent.textContent += (logContent.textContent ? '\n' : '') + line;

				// Only auto-scroll if user hasn't manually scrolled up
				if (!isUserScrolling) {
					logContent.scrollTop = logContent.scrollHeight;
				}
			}
		};

		eventSource.addEventListener('complete', function(event) {
			console.log('Received complete event:', event.data);
				// Hide spinner
				if (spinnerContainer) spinnerContainer.style.display = 'none';

				if (event.data === 'show_apply') {
					// Show password prompt and apply button
					statusArea.innerHTML = '<div class="mt-4 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md">' +
						'<div class="space-y-4">' +
							'<div class="flex items-center gap-2 text-blue-800 dark:text-blue-200">' +
								'<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">' +
									'<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>' +
								'</svg>' +
								'<span class="font-medium">Review the changes above. Apply configuration?</span>' +
							'</div>' +
							'<div class="flex items-center gap-4">' +
								'<div class="flex-1">' +
									'<label for="sudo-password" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Sudo Password</label>' +
									'<input type="password" id="sudo-password" name="password" placeholder="Enter your sudo password" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-white" onkeypress="if(event.key===\'Enter\') applySudoUpdate()" />' +
								'</div>' +
								'<div class="tooltip">' +
									'<button onclick="applySudoUpdate()" class="btn btn-success mt-6">' +
										'<svg class="w-5 h-5 inline-block mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">' +
											'<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>' +
										'</svg>' +
										'Apply Changes' +
									'</button>' +
								'<span class="tooltip-text"><code>sudo nixos-rebuild switch</code></span>' +
							'</div>' +
						'</div>' +
					'</div>';

					document.getElementById('sudo-password').focus();

					window.applySudoUpdate = function() {
						const password = document.getElementById('sudo-password').value;
						if (!password) {
							alert('Please enter your sudo password');
							return;
						}

					// Show spinner and update text
					if (spinnerContainer) {
							spinnerContainer.style.display = 'flex';
							spinnerContainer.innerHTML = makeSpinner() + '<span class="text-sm font-medium">Applying configuration...</span>';
						}

						// Clear the status area
						statusArea.innerHTML = '';

						// Send the apply request
						fetch('/api/update/apply', {
							method: 'POST',
							headers: {
								'Content-Type': 'application/x-www-form-urlencoded',
							},
							body: 'password=' + encodeURIComponent(password)
						}).catch(err => {
							console.error('Apply request failed:', err);
							statusArea.innerHTML = '<div class="text-red-600">Failed to start apply: ' + err.message + '</div>';
						});
					};

					htmx.process(statusArea);
				} else if (event.data === 'flake_done') {
					// Show flake completion with rollback button
					statusArea.innerHTML = ` + "`" + `
						<div class="mt-4 p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-md">
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-2 text-green-800 dark:text-green-200">
									<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
									</svg>
									<span class="font-medium">Flake update completed successfully!</span>
								</div>
								<div class="tooltip">
									<button onclick="rollbackFlake()" class="btn btn-warning btn-sm">
										<svg class="w-4 h-4 inline-block mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6"></path>
										</svg>
										Rollback
									</button>
									<span class="tooltip-text"><code>git restore flake.lock</code></span>
								</div>
							</div>
						</div>` + "`" + `;
					eventSource.close();

					window.rollbackFlake = function() {
						if (!confirm('Rollback flake.lock using git restore? This will discard any uncommitted changes to flake.lock.')) {
							return;
						}

						fetch('/api/update/flake/rollback', {
							method: 'POST'
						})
						.then(response => {
							if (!response.ok) {
								return response.text().then(text => { throw new Error(text); });
							}
							return response.text();
						})
						.then(message => {
							alert(message);
							window.location.reload();
						})
						.catch(err => {
							alert('Rollback failed: ' + err.message);
						});
					};
				} else {
					// Show completion and close connection
					statusArea.innerHTML = ` + "`" + `
						<div class="mt-4 p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-md">
							<div class="flex items-center gap-2 text-green-800 dark:text-green-200">
								<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
								</svg>
								<span class="font-medium">Update completed successfully!</span>
							</div>
						</div>` + "`" + `;
					eventSource.close();
				}
			});

		eventSource.onerror = function(error) {
			console.error('SSE error:', error);
			eventSource.close();
		};
		})();
			`),
		),
	}))
}
