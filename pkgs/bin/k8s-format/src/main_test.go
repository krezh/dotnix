package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// copyTestdata copies a testdata fixture into a temp dir and returns the path.
func copyTestdata(t *testing.T, fixture string) string {
	t.Helper()
	src := filepath.Join("testdata", filepath.FromSlash(fixture))
	data, err := os.ReadFile(src)
	if err != nil {
		t.Fatalf("read testdata %s: %v", src, err)
	}
	dir := t.TempDir()
	dst := filepath.Join(dir, filepath.Base(src))
	if err := os.WriteFile(dst, data, 0644); err != nil {
		t.Fatalf("write temp file: %v", err)
	}
	return dst
}

func writeTemp(t *testing.T, filename, content string) string {
	t.Helper()
	dir := t.TempDir()
	path := filepath.Join(dir, filename)
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("write temp file: %v", err)
	}
	return path
}

func readFile(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	return string(data)
}

// schemaLine extracts the yaml-language-server comment from formatted output.
func schemaLine(content string) string {
	for _, line := range strings.SplitN(content, "\n", 5) {
		if strings.HasPrefix(line, "# yaml-language-server:") {
			return line
		}
	}
	return ""
}

func newStats() *Stats {
	return &Stats{ByKind: make(map[string]int), ByKindFmt: make(map[string]int)}
}

// ---- getSchemaURL unit tests ----

func TestGetSchemaURL(t *testing.T) {
	tests := []struct {
		actualKind string
		apiVersion string
		want       string
	}{
		{
			"HelmRelease", "helm.toolkit.fluxcd.io/v2",
			"https://k8s-schemas.home-operations.com/helm.toolkit.fluxcd.io/helmrelease_v2.json",
		},
		{
			"OCIRepository", "source.toolkit.fluxcd.io/v1beta2",
			"https://k8s-schemas.home-operations.com/source.toolkit.fluxcd.io/ocirepository_v1beta2.json",
		},
		{
			// Flux Kustomization (ks.yaml)
			"Kustomization", "kustomize.toolkit.fluxcd.io/v1",
			"https://k8s-schemas.home-operations.com/kustomize.toolkit.fluxcd.io/kustomization_v1.json",
		},
		{
			// Native kustomization.yaml — actual kind is Kustomization, different apiVersion
			"Kustomization", "kustomize.config.k8s.io/v1beta1",
			"https://json.schemastore.org/kustomization",
		},
		{
			"ExternalSecret", "external-secrets.io/v1beta1",
			"https://k8s-schemas.home-operations.com/external-secrets.io/externalsecret_v1beta1.json",
		},
		{"HelmRelease", "invalid", ""},
		{"HelmRelease", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.actualKind+"/"+tt.apiVersion, func(t *testing.T) {
			got := getSchemaURL(tt.actualKind, tt.apiVersion)
			if got != tt.want {
				t.Errorf("got  %q\nwant %q", got, tt.want)
			}
		})
	}
}

// TestSchemaExceptionPreserved: an app-template HelmRelease keeps the bjw-s-labs schema URL.
func TestSchemaExceptionPreserved(t *testing.T) {
	const customURL = "https://raw.githubusercontent.com/bjw-s-labs/helm-charts/main/charts/other/app-template/schemas/helmrelease-helm-v2.schema.json"

	content := "# yaml-language-server: $schema=" + customURL + "\n" + `---
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: test
  namespace: default
spec:
  interval: 30m
  chart:
    spec:
      chart: app-template
      version: 3.0.0
`
	path := writeTemp(t, "helmrelease.yaml", content)
	if err := formatYAMLFile(path, "HelmRelease", newStats(), nil, ""); err != nil {
		t.Fatalf("formatYAMLFile: %v", err)
	}

	got := schemaLine(readFile(t, path))
	want := "# yaml-language-server: $schema=" + customURL
	if got != want {
		t.Errorf("exception schema was overwritten:\ngot  %q\nwant %q", got, want)
	}
}

// ---- schema injection / update via formatYAMLFile ----

func TestSchemaInjected(t *testing.T) {
	tests := []struct {
		fixture      string
		expectedKind string
		wantSchema   string
	}{
		{
			"helmrelease/helmrelease.yaml", "HelmRelease",
			"# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/helm.toolkit.fluxcd.io/helmrelease_v2.json",
		},
		{
			"ocirepository/ocirepository.yaml", "OCIRepository",
			"# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/source.toolkit.fluxcd.io/ocirepository_v1beta2.json",
		},
		{
			"ks/ks.yaml", "Kustomization",
			"# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/kustomize.toolkit.fluxcd.io/kustomization_v1.json",
		},
		{
			"kustomization/kustomization.yaml", "KustomizationFile",
			"# yaml-language-server: $schema=https://json.schemastore.org/kustomization",
		},
		{
			"externalsecret/externalsecret.yaml", "ExternalSecret",
			"# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/external-secrets.io/externalsecret_v1beta1.json",
		},
	}

	for _, tt := range tests {
		t.Run(tt.fixture, func(t *testing.T) {
			path := copyTestdata(t, tt.fixture)
			if err := formatYAMLFile(path, tt.expectedKind, newStats(), nil, ""); err != nil {
				t.Fatalf("formatYAMLFile: %v", err)
			}

			got := schemaLine(readFile(t, path))
			if got != tt.wantSchema {
				t.Errorf("schema comment:\ngot  %q\nwant %q", got, tt.wantSchema)
			}
		})
	}
}

// TestSchemaUpdated: a file with a stale/wrong schema gets corrected.
func TestSchemaUpdated(t *testing.T) {
	const stale = "# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/helm.toolkit.fluxcd.io/helmrelease_v1.json"
	const want = "# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/helm.toolkit.fluxcd.io/helmrelease_v2.json"

	content := stale + "\n" + `---
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: test
  namespace: default
spec:
  interval: 30m
  chart:
    spec:
      chart: test
      version: 1.0.0
`
	path := writeTemp(t, "helmrelease.yaml", content)
	if err := formatYAMLFile(path, "HelmRelease", newStats(), nil, ""); err != nil {
		t.Fatalf("formatYAMLFile: %v", err)
	}

	got := schemaLine(readFile(t, path))
	if got != want {
		t.Errorf("stale schema not corrected:\ngot  %q\nwant %q", got, want)
	}
}

// TestSchemaInjectedIdempotent: a second run on an already-correct file is a no-op.
func TestSchemaInjectedIdempotent(t *testing.T) {
	path := copyTestdata(t, "helmrelease/helmrelease.yaml")

	if err := formatYAMLFile(path, "HelmRelease", newStats(), nil, ""); err != nil {
		t.Fatalf("first run: %v", err)
	}
	after1 := readFile(t, path)

	if err := formatYAMLFile(path, "HelmRelease", newStats(), nil, ""); err != nil {
		t.Fatalf("second run: %v", err)
	}
	after2 := readFile(t, path)

	if after1 != after2 {
		t.Errorf("second run changed the file:\nbefore:\n%s\nafter:\n%s", after1, after2)
	}
}

// ---- spec.path normalization ----

func TestKsPathNormalized(t *testing.T) {
	content := `---
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: test
  namespace: flux-system
spec:
  interval: 30m
  path: clusters/dextek/apps/test/app
  prune: true
  sourceRef:
    kind: OCIRepository
    name: flux-system
`
	path := writeTemp(t, "ks.yaml", content)
	if err := formatYAMLFile(path, "Kustomization", newStats(), nil, ""); err != nil {
		t.Fatalf("formatYAMLFile: %v", err)
	}

	out := readFile(t, path)
	if !strings.Contains(out, "path: ./clusters/dextek/apps/test/app") {
		t.Errorf("spec.path not normalized, got:\n%s", out)
	}
}

func TestKsPathAlreadyNormalized(t *testing.T) {
	content := `---
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: test
  namespace: flux-system
spec:
  interval: 30m
  path: ./clusters/dextek/apps/test/app
  prune: true
  sourceRef:
    kind: OCIRepository
    name: flux-system
`
	path := writeTemp(t, "ks.yaml", content)
	before := content

	if err := formatYAMLFile(path, "Kustomization", newStats(), nil, ""); err != nil {
		t.Fatalf("formatYAMLFile: %v", err)
	}

	// Path should be unchanged; only schema injected
	out := readFile(t, path)
	if strings.Contains(out, "path: .././") || strings.Contains(out, "path: ././") {
		t.Errorf("path double-prefixed:\n%s", out)
	}
	_ = before
}

// TestAppTemplateSchemaDetected: HelmRelease with a chartRef pointing to an OCIRepository
// whose URL ends in /app-template gets the bjw-s-labs schema injected automatically.
func TestAppTemplateSchemaDetected(t *testing.T) {
	dir := t.TempDir()

	ociContent := `---
apiVersion: source.toolkit.fluxcd.io/v1beta2
kind: OCIRepository
metadata:
  name: myapp
spec:
  interval: 30m
  url: oci://ghcr.io/bjw-s-labs/helm/app-template
  ref:
    tag: 3.0.0
`
	hrContent := `---
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: myapp
  namespace: default
spec:
  interval: 30m
  chartRef:
    kind: OCIRepository
    name: myapp
  values:
    controllers:
      app:
        containers:
          app:
            image:
              repository: ghcr.io/example/myapp
              tag: latest
`
	ociPath := filepath.Join(dir, "ocirepository.yaml")
	hrPath := filepath.Join(dir, "helmrelease.yaml")
	if err := os.WriteFile(ociPath, []byte(ociContent), 0644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(hrPath, []byte(hrContent), 0644); err != nil {
		t.Fatal(err)
	}

	ociIndex := buildOCIRepoIndex(dir)
	if err := formatYAMLFile(hrPath, "HelmRelease", newStats(), ociIndex, dir); err != nil {
		t.Fatalf("formatYAMLFile: %v", err)
	}

	const wantSchema = "# yaml-language-server: $schema=https://raw.githubusercontent.com/bjw-s-labs/helm-charts/main/charts/other/app-template/schemas/helmrelease-helm-v2.schema.json"
	got := schemaLine(readFile(t, hrPath))
	if got != wantSchema {
		t.Errorf("app-template schema not injected:\ngot  %q\nwant %q", got, wantSchema)
	}
}

// TestKsPathMissingPrefixFixed: spec.path is missing the leading directory segment
// (e.g. ./dextek/apps/ai/llmkube/models instead of ./clusters/dextek/apps/ai/llmkube/models).
// The tool resolves the correct path by matching the suffix against the ks.yaml location.
func TestKsPathMissingPrefixFixed(t *testing.T) {
	// Build a tree: <root>/clusters/dextek/apps/ai/llmkube/{ks.yaml, models/}
	root := t.TempDir()
	appDir := filepath.Join(root, "clusters", "dextek", "apps", "ai", "llmkube")
	modelsDir := filepath.Join(appDir, "models")
	if err := os.MkdirAll(modelsDir, 0755); err != nil {
		t.Fatal(err)
	}

	content := `---
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: llmkube-models
  namespace: flux-system
spec:
  interval: 1h
  path: ./dextek/apps/ai/llmkube/models
  prune: true
  sourceRef:
    kind: OCIRepository
    name: flux-system
`
	ksPath := filepath.Join(appDir, "ks.yaml")
	if err := os.WriteFile(ksPath, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}

	if err := formatYAMLFile(ksPath, "Kustomization", newStats(), nil, root); err != nil {
		t.Fatalf("formatYAMLFile: %v", err)
	}

	out := readFile(t, ksPath)
	if !strings.Contains(out, "path: ./clusters/dextek/apps/ai/llmkube/models") {
		t.Errorf("path not corrected:\n%s", out)
	}
}

// TestKsPathMultiDocFix mirrors the llmkube case: two-document ks.yaml where
// the first doc has a correct path and the second has a missing prefix.
func TestKsPathMultiDocFix(t *testing.T) {
	root := t.TempDir()
	appDir := filepath.Join(root, "clusters", "dextek", "apps", "ai", "llmkube")
	if err := os.MkdirAll(filepath.Join(appDir, "app"), 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(appDir, "models"), 0755); err != nil {
		t.Fatal(err)
	}

	content := `---
# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/kustomize.toolkit.fluxcd.io/kustomization_v1.json
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: llmkube
  namespace: flux-system
spec:
  targetNamespace: ai
  path: ./clusters/dextek/apps/ai/llmkube/app
  prune: true
  sourceRef:
    kind: OCIRepository
    name: flux-system
  interval: 1h
---
# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/kustomize.toolkit.fluxcd.io/kustomization_v1.json
apiVersion: kustomize.toolkit.fluxcd.io/v1
kind: Kustomization
metadata:
  name: llmkube-models
  namespace: flux-system
spec:
  targetNamespace: ai
  path: ./dextek/apps/ai/llmkube/models
  prune: true
  sourceRef:
    kind: OCIRepository
    name: flux-system
  interval: 1h
`
	ksPath := filepath.Join(appDir, "ks.yaml")
	if err := os.WriteFile(ksPath, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}

	if err := formatYAMLFile(ksPath, "Kustomization", newStats(), nil, root); err != nil {
		t.Fatalf("formatYAMLFile: %v", err)
	}

	out := readFile(t, ksPath)
	if !strings.Contains(out, "path: ./clusters/dextek/apps/ai/llmkube/models") {
		t.Errorf("second doc path not corrected:\n%s", out)
	}
	if !strings.Contains(out, "path: ./clusters/dextek/apps/ai/llmkube/app") {
		t.Errorf("first doc path was changed unexpectedly:\n%s", out)
	}
}

// TestNonAppTemplateSchemaNotDetected: HelmRelease with a chartRef that does NOT point to
// app-template gets the standard derived schema, not the bjw-s-labs one.
func TestNonAppTemplateSchemaNotDetected(t *testing.T) {
	dir := t.TempDir()

	ociContent := `---
apiVersion: source.toolkit.fluxcd.io/v1beta2
kind: OCIRepository
metadata:
  name: myapp
spec:
  interval: 30m
  url: oci://ghcr.io/someorg/helm/myapp
  ref:
    tag: 1.0.0
`
	hrContent := `---
apiVersion: helm.toolkit.fluxcd.io/v2
kind: HelmRelease
metadata:
  name: myapp
  namespace: default
spec:
  interval: 30m
  chartRef:
    kind: OCIRepository
    name: myapp
  values: {}
`
	ociPath := filepath.Join(dir, "ocirepository.yaml")
	hrPath := filepath.Join(dir, "helmrelease.yaml")
	if err := os.WriteFile(ociPath, []byte(ociContent), 0644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(hrPath, []byte(hrContent), 0644); err != nil {
		t.Fatal(err)
	}

	ociIndex := buildOCIRepoIndex(dir)
	if err := formatYAMLFile(hrPath, "HelmRelease", newStats(), ociIndex, dir); err != nil {
		t.Fatalf("formatYAMLFile: %v", err)
	}

	const wantSchema = "# yaml-language-server: $schema=https://k8s-schemas.home-operations.com/helm.toolkit.fluxcd.io/helmrelease_v2.json"
	got := schemaLine(readFile(t, hrPath))
	if got != wantSchema {
		t.Errorf("wrong schema injected:\ngot  %q\nwant %q", got, wantSchema)
	}
}
