package main

// ResourceType defines formatting rules for a Kubernetes resource type
type ResourceType struct {
	Kind      string              // The Kubernetes Kind
	Filenames []string            // Filenames that contain this resource type
	Ordering  map[string][]string // Field ordering rules (metadata, spec, values, etc.)
}

// schemaBaseURL is the base for auto-derived yaml-language-server schema comments.
// Schema URLs follow the pattern: schemaBaseURL/{group}/{kind}_{version}.json
const schemaBaseURL = "https://kubernetes-schemas.plexuz.xyz"

// appTemplateSchemaURLs lists known schema URLs for bjw-s app-template HelmReleases.
// Files carrying one of these URLs are left alone unless app-template is positively detected.
var appTemplateSchemaURLs = []string{
	"https://raw.githubusercontent.com/bjw-s-labs/helm-charts/main/charts/other/app-template/schemas/helmrelease-helm-v2.schema.json",
}

// resourceTypes is the list of known Kubernetes resource kinds with field ordering rules.
// Add new resource types here to extend formatting support.
var resourceTypes = []ResourceType{
	{
		Kind:      "HelmRelease",
		Filenames: []string{"helmrelease.yaml"},
		Ordering: map[string][]string{
			"metadata": {"name", "namespace", "labels", "annotations"},
			"spec":     {"interval", "chartRef", "chart", "driftDetection", "install", "upgrade", "uninstall", "dependsOn", "timeout", "maxHistory", "valuesFrom", "values", "postRenderers"},
			"values":   {"global", "defaultPodOptions", "controllers", "service", "ingress", "route", "persistence", "configMaps", "secrets", "serviceAccount", "rbac"},
		},
	},
	{
		Kind:      "OCIRepository",
		Filenames: []string{"ocirepository.yaml"},
		Ordering: map[string][]string{
			"metadata": {"name", "namespace", "labels", "annotations"},
			"spec":     {"interval", "layerSelector", "ref", "url", "insecure", "provider", "timeout"},
		},
	},
	{
		Kind:      "Kustomization",
		Filenames: []string{"ks.yaml"},
		Ordering: map[string][]string{
			"metadata": {"name", "namespace", "labels", "annotations"},
			"spec":     {"targetNamespace", "commonMetadata", "dependsOn", "path", "prune", "sourceRef", "wait", "interval", "retryInterval", "timeout", "force", "components", "postBuild", "patches", "images"},
		},
	},
	{
		Kind:      "KustomizationFile",
		Filenames: []string{"kustomization.yaml"},
		Ordering: map[string][]string{
			"root": {"apiVersion", "kind", "resources"},
		},
	},
	{
		Kind:      "ExternalSecret",
		Filenames: []string{"externalsecret.yaml"},
		Ordering: map[string][]string{
			"metadata": {"name", "namespace", "labels", "annotations"},
			"spec":     {"refreshInterval", "secretStoreRef", "target", "data", "dataFrom"},
		},
	},
	{
		Kind:      "GrafanaDashboard",
		Filenames: []string{"grafanadashboard.yaml"},
		Ordering: map[string][]string{
			"metadata": {"name", "namespace", "labels", "annotations"},
			"spec":     {"allowCrossNamespaceImport", "instanceSelector", "datasources", "url"},
		},
	},
}

// schemaOverrides maps "apiVersion kind" to a fixed schema URL, bypassing auto-derivation.
var schemaOverrides = map[string]string{
	"kustomize.config.k8s.io/v1beta1 Kustomization": "https://json.schemastore.org/kustomization",
}

// schemaExclusions lists apiVersions or "apiVersion kind" pairs that should never receive a
// schema comment. An entry with just an apiVersion (e.g. "apps/v1") excludes all kinds under
// that group. An entry with a kind (e.g. "apps/v1 Deployment") excludes only that specific kind.
var schemaExclusions = map[string]bool{
	"apps/v1": true, // all Deployments, StatefulSets, DaemonSets, etc.
	"v1":      true, // all core resources (ConfigMap, Secret, Service, ...)
}

// fieldsToRemove lists fields to strip from resources (format: "field.path" or "Kind:field.path").
var fieldsToRemove = []string{
	"HelmRelease:spec.maxHistory",
	"HelmRelease:spec.uninstall",
}

// rootLevelFields are the top-level fields directly under apiVersion/kind in a Kubernetes resource.
// Any ordering key NOT in this list is treated as a nested field.
var rootLevelFields = map[string]bool{
	"metadata":   true,
	"spec":       true,
	"data":       true,
	"stringData": true,
	"status":     true,
}

// Build lookup maps from resourceTypes
var (
	fieldOrdering   map[string]map[string][]string
	fileKindMap     map[string]string
	resourceTypeMap map[string]ResourceType
)

func init() {
	fieldOrdering = make(map[string]map[string][]string)
	fileKindMap = make(map[string]string)
	resourceTypeMap = make(map[string]ResourceType)

	for _, rt := range resourceTypes {
		fieldOrdering[rt.Kind] = rt.Ordering
		resourceTypeMap[rt.Kind] = rt
		for _, filename := range rt.Filenames {
			fileKindMap[filename] = rt.Kind
		}
	}
}
