package templates

import (
	"strings"

	g "maragu.dev/gomponents"
)

// HxPost creates an hx-post attribute for HTMX POST requests.
func HxPost(url string) g.Node {
	return g.Attr("hx-post", url)
}

// HxGet creates an hx-get attribute for HTMX GET requests.
func HxGet(url string) g.Node {
	return g.Attr("hx-get", url)
}

// HxSwap creates an hx-swap attribute to control how content is swapped.
func HxSwap(swap string) g.Node {
	return g.Attr("hx-swap", swap)
}

// HxTarget creates an hx-target attribute to specify the swap target.
func HxTarget(target string) g.Node {
	return g.Attr("hx-target", target)
}

// HxConfirm creates an hx-confirm attribute to show a confirmation dialog.
func HxConfirm(msg string) g.Node {
	return g.Attr("hx-confirm", msg)
}

// HxTrigger creates an hx-trigger attribute to control when requests are triggered.
func HxTrigger(trigger string) g.Node {
	return g.Attr("hx-trigger", trigger)
}

// HxBoost creates an hx-boost attribute for progressive enhancement.
func HxBoost(boost string) g.Node {
	return g.Attr("hx-boost", boost)
}

// ClassNames conditionally joins class names based on a map of conditions.
// Base class is always included, conditional classes are added when their value is true.
func ClassNames(base string, conditionals map[string]bool) string {
	classes := []string{base}
	for class, include := range conditionals {
		if include {
			classes = append(classes, class)
		}
	}
	return strings.Join(classes, " ")
}

// DataAttr creates a data-* attribute.
func DataAttr(name, value string) g.Node {
	return g.Attr("data-"+name, value)
}
