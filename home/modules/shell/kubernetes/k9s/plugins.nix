{
  toggle-helmrelease = {
    shortCut = "Shift-T";
    confirm = true;
    scopes = [ "helmreleases" ];
    description = "Toggle to suspend or resume a HelmRelease";
    command = "bash";
    background = true;
    args = [
      "-c"
      "suspended=$(kubectl --context $CONTEXT get helmreleases -n $NAMESPACE $NAME -o=custom-columns=TYPE:.spec.suspend | tail -1); verb=$([ $suspended = \"true\" ] && echo \"resume\" || echo \"suspend\"); flux $verb helmrelease --context $CONTEXT -n $NAMESPACE $NAME"
    ];
  };
  toggle-kustomization = {
    shortCut = "Shift-T";
    confirm = true;
    scopes = [ "kustomizations" ];
    description = "Toggle to suspend or resume a Kustomization";
    command = "bash";
    background = true;
    args = [
      "-c"
      "suspended=$(kubectl --context $CONTEXT get kustomizations -n $NAMESPACE $NAME -o=custom-columns=TYPE:.spec.suspend | tail -1); verb=$([ $suspended = \"true\" ] && echo \"resume\" || echo \"suspend\"); flux $verb kustomization --context $CONTEXT -n $NAMESPACE $NAME"
    ];
  };
  reconcile-git = {
    shortCut = "Shift-R";
    confirm = false;
    description = "Flux reconcile";
    scopes = [ "gitrepositories" ];
    command = "bash";
    background = true;
    args = [
      "-c"
      "flux reconcile source git --context $CONTEXT -n $NAMESPACE $NAME"
    ];
  };
  reconcile-hr = {
    shortCut = "Shift-R";
    confirm = false;
    description = "Flux reconcile";
    scopes = [ "helmreleases" ];
    command = "bash";
    background = true;
    args = [
      "-c"
      "flux reconcile helmrelease --context $CONTEXT -n $NAMESPACE $NAME"
    ];
  };
  reconcile-helm-repo = {
    shortCut = "Shift-R";
    description = "Flux reconcile";
    scopes = [ "helmrepositories" ];
    command = "bash";
    background = true;
    confirm = false;
    args = [
      "-c"
      "flux reconcile source helm --context $CONTEXT -n $NAMESPACE $NAME"
    ];
  };
  reconcile-oci-repo = {
    shortCut = "Shift-R";
    description = "Flux reconcile";
    scopes = [ "ocirepositories" ];
    command = "bash";
    background = true;
    confirm = false;
    args = [
      "-c"
      "flux reconcile source oci --context $CONTEXT -n $NAMESPACE $NAME"
    ];
  };
  reconcile-ks = {
    shortCut = "Shift-R";
    confirm = false;
    description = "Flux reconcile";
    scopes = [ "kustomizations" ];
    command = "bash";
    background = true;
    args = [
      "-c"
      "flux reconcile kustomization --context $CONTEXT -n $NAMESPACE $NAME"
    ];
  };
  trace = {
    shortCut = "Shift-P";
    confirm = false;
    description = "Flux trace";
    scopes = [ "all" ];
    command = "bash";
    background = false;
    args = [
      "-c"
      "if [ -n \"$RESOURCE_GROUP\" ]; then api_endpoint=\"/apis/$RESOURCE_GROUP/$RESOURCE_VERSION\"; else api_endpoint=\"/api/$RESOURCE_VERSION\"; fi; api_resource=$(kubectl get --raw \"\${api_endpoint}\" | jq -r \".resources[] | select(.name==\\\"$RESOURCE_NAME\\\")\"); kind=$(echo \${api_resource} | jq -r '.kind'); namespace_arg=$(echo \${api_resource} | jq -r \"if .namespaced == true then \\\"--namespace $NAMESPACE\\\" else \\\"\\\" end\"); [ -n \"$RESOURCE_GROUP\" ] && api_version=$RESOURCE_GROUP/; api_version=\${api_version}$RESOURCE_VERSION; flux trace --context $CONTEXT --kind \${kind} --api-version \${api_version} \${namespace_arg} $NAME |& less -K"
    ];
  };
  get-suspended-helmreleases = {
    shortCut = "Shift-S";
    confirm = false;
    description = "Suspended Helm Releases";
    scopes = [ "helmrelease" ];
    command = "sh";
    background = false;
    args = [
      "-c"
      "kubectl get --context $CONTEXT --all-namespaces helmreleases.helm.toolkit.fluxcd.io -o json | jq -r '.items[] | select(.spec.suspend==true) | [.metadata.namespace,.metadata.name,.spec.suspend] | @tsv'"
    ];
  };
  get-suspended-kustomizations = {
    shortCut = "Shift-S";
    confirm = false;
    description = "Suspended Kustomizations";
    scopes = [ "kustomizations" ];
    command = "sh";
    background = false;
    args = [
      "-c"
      "kubectl get --context $CONTEXT --all-namespaces kustomizations.kustomize.toolkit.fluxcd.io -o json | jq -r '.items[] | select(.spec.suspend==true) | [.metadata.name,.spec.suspend] | @tsv'"
    ];
  };
}
