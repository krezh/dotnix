package templates

import (
	"io"
	"os"

	g "maragu.dev/gomponents"
	"maragu.dev/gomponents/html"
)

// doctype is a custom node that renders DOCTYPE + html element
type doctype struct {
	html g.Node
}

func (d doctype) Render(w io.Writer) error {
	if _, err := w.Write([]byte("<!DOCTYPE html>")); err != nil {
		return err
	}
	return d.html.Render(w)
}

// Base renders the master layout with sidebar navigation and content.
func Base(title string, content g.Node) g.Node {
	hostname, _ := os.Hostname()

	return doctype{
		html: g.El("html",
			html.Lang("en"),
			html.Class("dark"),
			html.Head(
				html.Meta(html.Charset("UTF-8")),
				html.Meta(html.Name("viewport"), html.Content("width=device-width, initial-scale=1.0")),
				html.TitleEl(g.Textf("%s - NixOS WebGUI", title)),
				html.Link(html.Rel("stylesheet"), html.Href("/static/css/output.css")),
				html.Script(html.Src("/static/js/htmx.min.js")),
				html.Script(
					g.Raw(`
				// Shows a dialog element by removing the 'hidden' class.
				function showDialog(id) { document.getElementById(id).classList.remove('hidden'); }
				// Hides a dialog element by adding the 'hidden' class.
				function hideDialog(id) { document.getElementById(id).classList.add('hidden'); }
				// Returns the HTML string for a spinner-7 element.
				function makeSpinner() { return '<div class="spinner-7"><div></div><div></div><div></div></div>'; }

				// Clean up page-specific resources before hx-boost navigates to a new page.
				// Only runs when the swap target is <body> (i.e. a full-page boost navigation),
				// not for partial HTMX swaps within a page.
				document.addEventListener('htmx:beforeSwap', function(event) {
					if (event.detail.target !== document.body) return;
					// Close the dashboard SSE connection if open.
					if (window._dashboardSSE) {
						window._dashboardSSE.close();
						window._dashboardSSE = null;
					}
					// Cancel all running graph animation frames.
					if (window._graphState) {
						Object.values(window._graphState).forEach(function(s) {
							if (s.rafId) cancelAnimationFrame(s.rafId);
						});
						window._graphState = null;
					}
					// Remove the floating graph tooltip if present.
					if (window._graphTooltip) {
						window._graphTooltip.remove();
						window._graphTooltip = null;
					}
				});
				`),
				),
			),
			html.Body(
				html.Class("h-screen flex"),
				g.Attr("hx-boost", "true"),
				g.El("aside",
					html.Class("w-64 bg-gray-800 text-white flex flex-col"),
					html.Div(
						html.Class("p-5 flex items-center gap-3"),
						html.Img(html.Src("/static/img/nixos.svg"), html.Class("w-8 h-8 shrink-0"), html.Alt("NixOS")),
						html.Span(
							html.Class("font-semibold text-white truncate"),
							g.Text(hostname),
						),
					),
					g.El("nav",
						html.Class("flex-1 px-4 space-y-2"),
						html.A(
							html.Href("/"),
							html.Class(ClassNames("nav-link", map[string]bool{"nav-link-active": title == "Dashboard"})),
							g.Raw(`<svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"></path>
						</svg>`),
							g.Text("Dashboard"),
						),
						html.A(
							html.Href("/services"),
							html.Class(ClassNames("nav-link", map[string]bool{"nav-link-active": title == "Services"})),
							g.Raw(`<svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
						</svg>`),
							g.Text("Services"),
						),
						html.A(
							html.Href("/update"),
							html.Class(ClassNames("nav-link", map[string]bool{"nav-link-active": title == "Update"})),
							g.Raw(`<svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
						</svg>`),
							g.Text("Update"),
							html.Span(
								HxGet("/api/update/git-status-dot"),
								HxTrigger("load"),
								HxSwap("outerHTML"),
							),
						),
						html.A(
							html.Href("/generations"),
							html.Class(ClassNames("nav-link", map[string]bool{"nav-link-active": title == "Generations"})),
							g.Raw(`<svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
						</svg>`),
							g.Text("Generations"),
						),
						html.A(
							html.Href("/store"),
							html.Class(ClassNames("nav-link", map[string]bool{"nav-link-active": title == "Nix Store"})),
							g.Raw(`<svg class="w-5 h-5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
						</svg>`),
							g.Text("Nix Store"),
						),
					),
					html.Div(
						html.Class("p-4 border-t border-gray-700"),
						html.Div(
							html.Class("text-sm text-gray-400"),
							html.Div(g.Text("Version 0.1.0")),
							html.Div(
								html.Class("mt-1"),
								g.Text("NixOS Management"),
							),
						),
					),
				),
				html.Main(
					html.Class("flex-1 overflow-auto"),
					html.Div(
						html.Class("container mx-auto p-8"),
						html.H2(
							html.Class("text-3xl font-bold mb-6"),
							g.Text(title),
						),
						content,
					),
				),
			),
		),
	}
}
