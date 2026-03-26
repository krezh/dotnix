package templates

import (
	"fmt"

	g "github.com/maragudk/gomponents"
	"github.com/maragudk/gomponents/html"

	"github.com/krezh/nixos-webgui/system"
)

// Services renders the service management page.
func Services(services []system.Service, filter string) g.Node {
	return Base("Services", ServicesContent(services, filter))
}

// ServicesContent renders the service management content with filters and service table.
func ServicesContent(services []system.Service, filter string) g.Node {
	return g.Group([]g.Node{
		html.Div(
			html.Class("mb-6"),
			html.Div(
				html.Class("flex gap-4"),
				html.Div(
					html.Class("tooltip"),
					html.A(
						html.Href("/services?filter=all"),
						html.Class(ClassNames("btn", map[string]bool{
							"btn-primary":   filter == "all" || filter == "",
							"btn-secondary": filter != "all" && filter != "",
						})),
						g.Text("All Services"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("systemctl list-units --type=service --all")),
					),
				),
				html.Div(
					html.Class("tooltip"),
					html.A(
						html.Href("/services?filter=active"),
						html.Class(ClassNames("btn", map[string]bool{
							"btn-primary":   filter == "active",
							"btn-secondary": filter != "active",
						})),
						g.Text("Active"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("systemctl list-units --type=service --state=active")),
					),
				),
				html.Div(
					html.Class("tooltip"),
					html.A(
						html.Href("/services?filter=failed"),
						html.Class(ClassNames("btn", map[string]bool{
							"btn-primary":   filter == "failed",
							"btn-secondary": filter != "failed",
						})),
						g.Text("Failed"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("systemctl list-units --type=service --state=failed")),
					),
				),
				html.Div(
					html.Class("tooltip"),
					html.A(
						html.Href("/services?filter=running"),
						html.Class(ClassNames("btn", map[string]bool{
							"btn-primary":   filter == "running",
							"btn-secondary": filter != "running",
						})),
						g.Text("Running"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("Running services")),
					),
				),
				html.Div(
					html.Class("tooltip"),
					html.A(
						html.Href("/services?filter=stopped"),
						html.Class(ClassNames("btn", map[string]bool{
							"btn-primary":   filter == "stopped",
							"btn-secondary": filter != "stopped",
						})),
						g.Text("Stopped"),
					),
					html.Span(
						html.Class("tooltip-text"),
						html.Code(g.Text("Stopped services")),
					),
				),
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
								g.Text("Service"),
							),
							html.Th(
								html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
								g.Text("Status"),
							),
							html.Th(
								html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
								g.Text("State"),
							),
							html.Th(
								html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
								g.Text("Enabled"),
							),
							html.Th(
								html.Class("px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"),
								g.Text("Actions"),
							),
						),
					),
					html.TBody(
						html.Class("bg-white divide-y divide-gray-200 dark:bg-gray-900 dark:divide-gray-800"),
						g.Group(g.Map(services, func(service system.Service) g.Node {
							return html.Tr(
								html.Class("table-row"),
								html.Td(
									html.Class("px-6 py-4 whitespace-nowrap"),
									html.Div(
										html.Class("text-sm font-medium"),
										g.Text(service.Name),
									),
									g.If(service.Description != "",
										html.Div(
											html.Class("text-xs text-gray-500 dark:text-gray-400"),
											g.Text(service.Description),
										),
									),
								),
								html.Td(
									html.Class("px-6 py-4 whitespace-nowrap"),
									StatusBadge(service.Status),
								),
								html.Td(
									html.Class("px-6 py-4 whitespace-nowrap"),
									g.If(service.Running, html.Span(html.Class("badge badge-success"), g.Text("Running"))),
									g.If(!service.Running, html.Span(html.Class("badge badge-info"), g.Text("Stopped"))),
								),
								html.Td(
									html.Class("px-6 py-4 whitespace-nowrap"),
									g.If(service.Enabled, html.Span(html.Class("badge badge-success"), g.Text("Enabled"))),
									g.If(!service.Enabled, html.Span(html.Class("badge badge-warning"), g.Text("Disabled"))),
								),
								html.Td(
									html.Class("px-6 py-4 whitespace-nowrap text-sm font-medium space-x-2"),
									g.If(service.Running,
										html.Div(
											html.Class("tooltip"),
											html.Button(
												g.Attr("onclick", fmt.Sprintf("showServiceAction('stop', '%s')", service.Name)),
												html.Class("btn btn-sm btn-danger"),
												g.Text("Stop"),
											),
											html.Span(
												html.Class("tooltip-text"),
												html.Code(g.Text("systemctl stop")),
											),
										),
									),
									g.If(!service.Running,
										html.Div(
											html.Class("tooltip"),
											html.Button(
												g.Attr("onclick", fmt.Sprintf("showServiceAction('start', '%s')", service.Name)),
												html.Class("btn btn-sm btn-success"),
												g.Text("Start"),
											),
											html.Span(
												html.Class("tooltip-text"),
												html.Code(g.Text("systemctl start")),
											),
										),
									),
									html.Div(
										html.Class("tooltip"),
										html.Button(
											g.Attr("onclick", fmt.Sprintf("showServiceAction('restart', '%s')", service.Name)),
											html.Class("btn btn-sm btn-secondary"),
											g.Text("Restart"),
										),
										html.Span(
											html.Class("tooltip-text"),
											html.Code(g.Text("systemctl restart")),
										),
									),
									html.Div(
										html.Class("tooltip"),
										html.Button(
											HxGet("/api/services/"+service.Name+"/logs"),
											HxTarget("#log-modal-content"),
											HxSwap("innerHTML"),
											g.Attr("onclick", "document.getElementById('log-modal').classList.remove('hidden')"),
											html.Class("btn btn-sm btn-secondary"),
											g.Text("Logs"),
										),
										html.Span(
											html.Class("tooltip-text"),
											html.Code(g.Text("journalctl -u [service] -n 100")),
										),
									),
								),
							)
						})),
					),
				),
			),
		),
		// Service Action Modals
		html.Div(
			html.ID("service-action-modal"),
			html.Class("hidden fixed inset-0 bg-gray-600 bg-opacity-50 flex items-center justify-center z-50"),
			html.Div(
				html.Class("bg-white dark:bg-gray-800 rounded-lg shadow-xl w-96 p-6"),
				html.H3(
					html.ID("action-modal-title"),
					html.Class("text-lg font-semibold mb-4"),
				),
				html.Form(
					html.ID("service-action-form"),
					html.Class("space-y-4"),
					PasswordInput("action-sudo-password"),
					html.Div(
						html.Class("flex gap-2 justify-end"),
						html.Button(
							html.Type("button"),
							g.Attr("onclick", "hideDialog('service-action-modal')"),
							html.Class("btn btn-secondary"),
							g.Text("Cancel"),
						),
						html.Button(
							html.Type("submit"),
							html.ID("action-submit-btn"),
							html.Class("btn btn-primary"),
							g.Text("Confirm"),
						),
					),
				),
			),
		),
		// Log Modal
		html.Div(
			html.ID("log-modal"),
			html.Class("hidden fixed inset-0 bg-gray-600 bg-opacity-50 flex items-center justify-center z-50"),
			html.Div(
				html.Class("bg-white dark:bg-gray-800 rounded-lg shadow-xl w-3/4 max-w-4xl max-h-[80vh] flex flex-col"),
				html.Div(
					html.Class("flex justify-between items-center p-6 border-b dark:border-gray-700"),
					html.H3(
						html.Class("text-xl font-semibold"),
						g.Text("Service Logs"),
					),
					html.Button(
						g.Attr("onclick", "document.getElementById('log-modal').classList.add('hidden')"),
						html.Class("text-gray-500 hover:text-gray-700"),
						g.Raw(`<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
						</svg>`),
					),
				),
				html.Div(
					html.ID("log-modal-content"),
					html.Class("p-6 overflow-auto flex-1"),
					g.Raw("<!-- Logs will be loaded here -->"),
				),
			),
		),
		html.Script(
			html.Type("text/javascript"),
			g.Raw(`
			let currentServiceAction = null;
			let currentServiceName = null;

			function showServiceAction(action, serviceName) {
				currentServiceAction = action;
				currentServiceName = serviceName;

				const title = document.getElementById('action-modal-title');
				const actionText = action.charAt(0).toUpperCase() + action.slice(1);
				title.textContent = actionText + ' Service: ' + serviceName;

				document.getElementById('action-sudo-password').value = '';
				showDialog('service-action-modal');
			}

			document.getElementById('service-action-form').addEventListener('submit', function(e) {
				e.preventDefault();
				const password = document.getElementById('action-sudo-password').value;
				const formData = new FormData();
				formData.append('password', password);

				const btn = document.getElementById('action-submit-btn');
				btn.disabled = true;
				btn.textContent = 'Processing...';

				fetch('/api/services/' + currentServiceName + '/' + currentServiceAction, {
					method: 'POST',
					body: formData
				})
				.then(response => {
					if (response.ok) {
						hideDialog('service-action-modal');
						location.reload();
					} else {
						return response.text().then(text => {
							alert('Error: ' + text);
						});
					}
				})
				.catch(error => {
					alert('Error: ' + error);
				})
				.finally(() => {
					btn.disabled = false;
					btn.textContent = 'Confirm';
				});
			});
			`),
		),
	})
}

// StatusBadge renders a status badge for a service.
func StatusBadge(status system.ServiceStatus) g.Node {
	switch status {
	case system.StatusActive:
		return html.Span(
			html.Class("badge badge-success"),
			g.Text("Active"),
		)
	case system.StatusFailed:
		return html.Span(
			html.Class("badge badge-danger"),
			g.Text("Failed"),
		)
	case system.StatusInactive:
		return html.Span(
			html.Class("badge badge-info"),
			g.Text("Inactive"),
		)
	default:
		return html.Span(
			html.Class("badge badge-warning"),
			g.Text("Unknown"),
		)
	}
}

// ServiceLogs renders the service logs view with follow functionality.
func ServiceLogs(serviceName string, logs string) g.Node {
	return html.Div(g.Group([]g.Node{
		html.Div(
			html.Class("space-y-4"),
			html.Div(
				html.Class("flex gap-2"),
				html.Button(
					html.ID("log-follow-btn"),
					g.Attr("onclick", "toggleLogFollow()"),
					html.Class("btn btn-sm btn-primary"),
					g.Text("Follow Logs"),
				),
				html.Button(
					g.Attr("onclick", "document.getElementById('log-output').innerHTML = ''"),
					html.Class("btn btn-sm btn-secondary"),
					g.Text("Clear"),
				),
			),
			html.Pre(
				html.ID("log-output"),
				html.Class("log-output"),
				g.Text(logs),
			),
		),
		html.Script(
			html.Type("text/javascript"),
			g.Raw(`
		(function() {
			let logEventSource = null;
			let isFollowing = false;
			const serviceName = `+serviceName+`;

			window.toggleLogFollow = function() {
				const btn = document.getElementById('log-follow-btn');
				const output = document.getElementById('log-output');

				if (isFollowing) {
					// Stop following
					if (logEventSource) {
						logEventSource.close();
						logEventSource = null;
					}
					btn.textContent = 'Follow Logs';
					btn.classList.remove('btn-danger');
					btn.classList.add('btn-primary');
					isFollowing = false;
				} else {
					// Start following
					output.innerHTML = '';
					logEventSource = new EventSource('/api/services/' + serviceName + '/logs/stream');

					logEventSource.onmessage = function(event) {
						const line = document.createElement('div');
						line.textContent = event.data;
						output.appendChild(line);
						// Auto-scroll to bottom
						output.scrollTop = output.scrollHeight;
					};

					logEventSource.onerror = function(error) {
						console.error('Log stream error:', error);
						logEventSource.close();
						logEventSource = null;
						btn.textContent = 'Follow Logs';
						btn.classList.remove('btn-danger');
						btn.classList.add('btn-primary');
						isFollowing = false;
					};

					btn.textContent = 'Stop Following';
					btn.classList.remove('btn-primary');
					btn.classList.add('btn-danger');
					isFollowing = true;
				}
			};

			// Clean up when modal closes
			const closeBtn = document.querySelector('#log-modal button[onclick*="hidden"]');
			if (closeBtn) {
				closeBtn.addEventListener('click', function() {
					if (logEventSource) {
						logEventSource.close();
						logEventSource = null;
					}
					isFollowing = false;
					const btn = document.getElementById('log-follow-btn');
					if (btn) {
						btn.textContent = 'Follow Logs';
						btn.classList.remove('btn-danger');
						btn.classList.add('btn-primary');
					}
				});
			}
		})();
			`),
		),
	}))
}
