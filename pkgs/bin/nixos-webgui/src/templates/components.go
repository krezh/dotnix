package templates

import (
	g "maragu.dev/gomponents"
	"maragu.dev/gomponents/html"

	"github.com/krezh/nixos-webgui/system"
)

// GitStatusDot renders a compact commit indicator for the nav link when commits are available.
// Renders nothing when behind is 0.
func GitStatusDot(behind int) g.Node {
	if behind > 0 {
		return html.Span(
			html.Class("ml-auto inline-flex items-center gap-1 px-1.5 py-0.5 rounded-full text-xs font-medium bg-blue-500/20 text-blue-300"),
			g.Raw(`<svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16V4m0 0L3 8m4-4l4 4m6 4v8m0 0l4-4m-4 4l-4-4"></path>
			</svg>`),
			g.Textf("%d", behind),
		)
	}
	return g.Text("")
}

// GitStatusIndicator renders a badge showing how many remote commits are available to pull.
// Shows a tooltip with commit messages on hover. Renders nothing when status.Behind is 0.
func GitStatusIndicator(status *system.GitStatus) g.Node {
	return html.Div(
		html.ID("git-status-indicator"),
		g.If(
			status != nil && status.Behind > 0,
			html.Div(
				html.Class("tooltip"),
				html.Span(
					html.Class("inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-300 cursor-default"),
					g.Raw(`<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16V4m0 0L3 8m4-4l4 4m6 4v8m0 0l4-4m-4 4l-4-4"></path>
					</svg>`),
					g.If(status.Behind == 1, g.Textf("%d new commit", status.Behind)),
					g.If(status.Behind != 1, g.Textf("%d new commits", status.Behind)),
				),
				html.Span(
					html.Class("tooltip-text-list"),
					g.Group(g.Map(status.Commits, func(msg string) g.Node {
						return html.Div(
							html.Class("py-0.5"),
							g.Text(msg),
						)
					})),
				),
			),
		),
	)
}

// CardHeading renders a standardized card section heading.
func CardHeading(title string) g.Node {
	return html.H3(
		html.Class("text-lg font-semibold mb-4"),
		g.Text(title),
	)
}

// PasswordInput renders a standard sudo password input field without a keypress handler.
// Attach onkeypress in the calling template if needed.
func PasswordInput(id string) g.Node {
	return html.Div(
		html.Label(
			html.Class("block text-sm font-medium mb-1"),
			g.Text("Sudo Password"),
		),
		html.Input(
			html.Type("password"),
			html.ID(id),
			html.Placeholder("Enter your sudo password"),
			html.Class("w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md dark:bg-gray-700 dark:text-white"),
		),
	)
}

// ModalDialog renders a centered dialog overlay with a title and body content.
// Use showDialog(id) / hideDialog(id) from JS to toggle visibility.
func ModalDialog(id string, title string, body g.Node) g.Node {
	return html.Div(
		html.ID(id),
		html.Class("hidden fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"),
		html.Div(
			html.Class("relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white dark:bg-gray-800"),
			html.Div(
				html.Class("mt-3"),
				html.H3(
					html.Class("text-lg font-medium leading-6 mb-4"),
					g.Text(title),
				),
				body,
			),
		),
	)
}
