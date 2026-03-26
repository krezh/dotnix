package templates

import (
	"fmt"

	g "maragu.dev/gomponents"
	"maragu.dev/gomponents/html"

	"github.com/krezh/nixos-webgui/system"
)

// Generations renders the system generations management page.
func Generations(generations []system.Generation) g.Node {
	return Base("Generations", GenerationsContent(generations))
}

// GenerationsContent renders the generations table and management dialogs.
func GenerationsContent(generations []system.Generation) g.Node {
	return g.Group([]g.Node{
		html.Div(
			html.Class("space-y-6"),
			html.Div(
				html.Class("card"),
				html.Div(
					html.Class("flex justify-between items-center mb-4"),
					CardHeading("System Generations"),
					html.Div(
						html.Class("tooltip"),
						html.Button(
							g.Attr("onclick", "showDialog('delete-dialog')"),
							html.Class("btn btn-danger btn-sm"),
							g.Raw(`<svg class="w-4 h-4 inline-block mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
							</svg>`),
							g.Text("Delete Old"),
						),
						html.Span(
							html.Class("tooltip-text"),
							html.Code(g.Text("sudo nix-env --delete-generations +N")),
						),
					),
				),
				html.P(
					html.Class("text-sm text-gray-600 dark:text-gray-400"),
					g.Text("Manage your NixOS system generations. You can rollback to previous configurations or delete old generations to free up disk space."),
				),
			),
			html.Div(
				html.Class("card"),
				html.Div(
					html.Class("overflow-x-auto"),
					html.Table(
						html.Class("table"),
						html.THead(
							html.Class("table-header"),
							html.Tr(
								html.Th(
									html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
									g.Text("Generation"),
								),
								html.Th(
									html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
									g.Text("Date"),
								),
								html.Th(
									html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
									g.Text("NixOS Version"),
								),
								html.Th(
									html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
									g.Text("Status"),
								),
								html.Th(
									html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
									g.Text("Actions"),
								),
							),
						),
						html.TBody(
							html.Class("divide-y divide-gray-200 dark:divide-gray-700"),
							g.Group(g.Map(generations, func(gen system.Generation) g.Node {
								return html.Tr(
									html.Class("table-row"),
									html.Td(
										html.Class("px-6 py-4 whitespace-nowrap"),
										html.Div(
											html.Class("text-sm font-medium"),
											g.Textf("#%d", gen.Number),
										),
									),
									html.Td(
										html.Class("px-6 py-4 whitespace-nowrap"),
										html.Div(
											html.Class("text-sm"),
											g.Text(gen.Date.Format("2006-01-02 15:04:05")),
										),
										html.Div(
											html.Class("text-xs text-gray-500"),
											g.Text(system.FormatRelativeTime(gen.Date)),
										),
									),
									html.Td(
										html.Class("px-6 py-4 whitespace-nowrap text-sm"),
										g.Text(gen.NixOSVersion),
									),
									html.Td(
										html.Class("px-6 py-4 whitespace-nowrap"),
										g.If(gen.Current,
											html.Span(
												html.Class("badge badge-success"),
												g.Text("Current"),
											),
										),
									),
									html.Td(
										html.Class("px-6 py-4 whitespace-nowrap text-sm"),
										g.If(!gen.Current,
											html.Div(
												html.Class("tooltip"),
												html.Button(
													html.Type("button"),
													html.Class("text-blue-600 hover:text-blue-900 dark:text-blue-400 dark:hover:text-blue-300"),
													DataAttr("gen", fmt.Sprintf("%d", gen.Number)),
													g.Attr("onclick", "rollbackToGeneration(parseInt(this.getAttribute('data-gen')))"),
													g.Text("Rollback"),
												),
												html.Span(
													html.Class("tooltip-text"),
													html.Code(g.Text("sudo nixos-rebuild switch --rollback")),
												),
											),
										),
									),
								)
							})),
						),
					),
				),
			),
		),
		// Delete Dialog
		html.Div(
			html.ID("delete-dialog"),
			html.Class("hidden fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"),
			html.Div(
				html.Class("relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white dark:bg-gray-800"),
				html.Div(
					html.Class("mt-3"),
					html.H3(
						html.Class("text-lg font-medium leading-6 mb-4"),
						g.Text("Delete Old Generations"),
					),
					html.Div(
						html.Class("mt-2 space-y-4"),
						html.Div(
							html.Label(
								html.Class("block text-sm font-medium mb-1"),
								g.Text("Keep Last N Generations"),
							),
							html.Input(
								html.Type("number"),
								html.ID("keep-last"),
								html.Value("5"),
								html.Min("1"),
								html.Class("w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md dark:bg-gray-700 dark:text-white"),
							),
							html.P(
								html.Class("text-xs text-gray-500 mt-1"),
								g.Text("Will delete all generations except the last N"),
							),
						),
						PasswordInput("delete-password"),
					),
					html.Div(
						html.Class("flex gap-3 mt-6"),
						html.Button(
							g.Attr("onclick", "confirmDelete()"),
							html.Class("btn btn-danger flex-1"),
							g.Text("Delete"),
						),
						html.Button(
							g.Attr("onclick", "hideDialog('delete-dialog')"),
							html.Class("btn btn-secondary flex-1"),
							g.Text("Cancel"),
						),
					),
				),
			),
		),
		// Rollback Dialog
		html.Div(
			html.ID("rollback-dialog"),
			html.Class("hidden fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"),
			html.Div(
				html.Class("relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white dark:bg-gray-800"),
				html.Div(
					html.Class("mt-3"),
					html.H3(
						html.Class("text-lg font-medium leading-6 mb-4"),
						g.Text("Rollback to Generation "),
						html.Span(html.ID("rollback-gen-num")),
					),
					html.Div(
						html.Class("mt-2 space-y-4"),
						html.P(
							html.Class("text-sm text-gray-600 dark:text-gray-400"),
							g.Text("This will switch your system to the selected generation. Your current configuration will remain available."),
						),
						PasswordInput("rollback-password"),
					),
					html.Div(
						html.Class("flex gap-3 mt-6"),
						html.Button(
							g.Attr("onclick", "confirmRollback()"),
							html.Class("btn btn-primary flex-1"),
							g.Text("Rollback"),
						),
						html.Button(
							g.Attr("onclick", "hideDialog('rollback-dialog')"),
							html.Class("btn btn-secondary flex-1"),
							g.Text("Cancel"),
						),
					),
				),
			),
		),
		html.Script(
			g.Raw(`
		let selectedGeneration = 0;

		// Wire up Enter key for password fields
		document.addEventListener('DOMContentLoaded', function() {
			var rp = document.getElementById('rollback-password');
			if (rp) rp.addEventListener('keypress', function(e) { if (e.key === 'Enter') confirmRollback(); });
			var dp = document.getElementById('delete-password');
			if (dp) dp.addEventListener('keypress', function(e) { if (e.key === 'Enter') confirmDelete(); });
		});

		function rollbackToGeneration(genNum) {
			selectedGeneration = genNum;
			document.getElementById('rollback-gen-num').textContent = '#' + genNum;
			showDialog('rollback-dialog');
		}

		function confirmRollback() {
			const password = document.getElementById('rollback-password').value;
			if (!password) {
				alert('Please enter your sudo password');
				return;
			}

			const dialog = document.getElementById('rollback-dialog');
			dialog.innerHTML = '<div class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white dark:bg-gray-800">' +
				'<div class="flex items-center justify-center gap-3 p-8">' +
					makeSpinner() +
					'<span>Rolling back...</span>' +
				'</div>' +
			'</div>';

			fetch('/api/generations/rollback', {
				method: 'POST',
				headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
				body: 'generation=' + selectedGeneration + '&password=' + encodeURIComponent(password)
			})
			.then(response => {
				if (response.ok) {
					alert('Rollback successful! The system has been switched to generation #' + selectedGeneration);
					window.location.reload();
				} else {
					return response.text().then(text => { throw new Error(text); });
				}
			})
			.catch(err => {
				alert('Rollback failed: ' + err.message);
				hideDialog('rollback-dialog');
			});
		}

		function confirmDelete() {
			const keepLast = document.getElementById('keep-last').value;
			const password = document.getElementById('delete-password').value;
			if (!password) {
				alert('Please enter your sudo password');
				return;
			}

			const dialog = document.getElementById('delete-dialog');
			dialog.innerHTML = '<div class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white dark:bg-gray-800">' +
				'<div class="flex items-center justify-center gap-3 p-8">' +
					makeSpinner() +
					'<span>Deleting old generations...</span>' +
				'</div>' +
			'</div>';

			fetch('/api/generations/delete', {
				method: 'POST',
				headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
				body: 'keep_last=' + keepLast + '&password=' + encodeURIComponent(password)
			})
			.then(response => {
				if (response.ok) {
					return response.text();
				} else {
					return response.text().then(text => { throw new Error(text); });
				}
			})
			.then(message => {
				alert(message);
				window.location.reload();
			})
			.catch(err => {
				alert('Delete failed: ' + err.message);
				hideDialog('delete-dialog');
			});
		}
			`),
		),
	})
}
