{ lib }:
let
  inherit (lib) elemAt length;

  toList = x: if lib.isList x then x else [ x ];

  # "a, b , c" → [ "a" "b" "c" ]
  fields = s: map lib.trim (lib.splitString "," s);

  # "0.05" → 0.05, "1" → 1, "1.0" → 1.0 (kept distinct so the Lua matches).
  num = builtins.fromJSON;

  # home-manager renders `settings.<name>` as `hl.<name>(<value>)`; an
  # `_args` attrset becomes a multi-argument call `hl.<name>(a, b, …)`.
  mkArgs = args: { _args = args; };

  # Expand any rule whose `match` has a list-valued field into one rule per
  # value (recursively, so several list fields produce the full product).
  expandRules =
    rules:
    lib.concatMap (
      rule:
      let
        m = rule.match or { };
        listField = lib.findFirst (n: lib.isList m.${n}) null (lib.attrNames m);
      in
      if listField == null then
        [ rule ]
      else
        expandRules (
          map (
            v:
            rule
            // {
              match = m // {
                ${listField} = v;
              };
            }
          ) m.${listField}
        )
    ) rules;

  # Build the whole window-rule list from one config.
  #
  #   tags.<name>.anyOf = { field = value; … }
  #       Each field on its own is enough to earn the tag (OR). A list value
  #       is also OR, one alternative per element.
  #   tags.<name>.allOf = [ { field = value; … } ]
  #       Every field of a set must hold together (AND). One set per way in.
  #   tags.<name>.apply = { …props }
  #       Applied to every window carrying the tag.
  #
  #   rules = [ { match = { … }; …props } ]
  #       Standalone rules, no tag. Fields inside `match` are AND. Order is
  #       kept, since Hyprland applies rules top-to-bottom.
  #
  # A list value anywhere in a `match` expands to one rule per element.
  mkRules =
    {
      tags ? { },
      rules ? [ ],
    }:
    let
      defs = lib.concatLists (
        lib.mapAttrsToList (
          tag: spec:
          lib.concatLists (
            lib.mapAttrsToList (
              field: vals:
              map (v: {
                match.${field} = v;
                tag = "+${tag}";
              }) (toList vals)
            ) (spec.anyOf or { })
          )
          ++ map (m: {
            match = m;
            tag = "+${tag}";
          }) (spec.allOf or [ ])
        ) tags
      );
      applies = lib.concatLists (
        lib.mapAttrsToList (
          tag: spec: lib.optional (spec ? apply) ({ match.tag = tag; } // spec.apply)
        ) tags
      );
    in
    expandRules (defs ++ applies ++ rules);

  # env = mkEnv { NAME = "value"; … } → hl.env("NAME", "value")
  mkEnv = lib.mapAttrsToList (
    name: value:
    mkArgs [
      name
      (toString value)
    ]
  );

  # curve = mkCurves { … } — one `hl.curve(name, spec)` per entry.
  #   bezier.<name> = "x1, y1, x2, y2"
  #   spring.<name> = "mass, stiffness, dampening"
  mkCurves =
    {
      bezier ? { },
      spring ? { },
    }:
    let
      parse =
        kind: count: build: name: spec:
        let
          n = map num (fields spec);
        in
        if length n != count then
          throw "hyprland: ${kind} \"${name}\" needs ${toString count} numbers, got \"${spec}\""
        else
          build n;

      beziers = lib.mapAttrs (parse "bezier" 4 (n: {
        type = "bezier";
        points = [
          [
            (elemAt n 0)
            (elemAt n 1)
          ]
          [
            (elemAt n 2)
            (elemAt n 3)
          ]
        ];
      })) bezier;

      springs = lib.mapAttrs (parse "spring" 3 (n: {
        type = "spring";
        mass = elemAt n 0;
        stiffness = elemAt n 1;
        dampening = elemAt n 2;
      })) spring;
    in
    lib.mapAttrsToList (
      name: spec:
      mkArgs [
        name
        spec
      ]
    ) (beziers // springs);

  # animation = mkAnimations [ … ], each entry
  #   "<leaf>, <speed>, bezier:<name>|spring:<name>[, <style>]"
  #   "<leaf>, off" disables the leaf.
  # Order is preserved on purpose: Hyprland applies leaves top-to-bottom and a
  # parent leaf resets its children, so this is a list rather than an attrset.
  mkAnimations = map (
    spec:
    let
      f = fields spec;
      leaf = elemAt f 0;
    in
    if
      f == [
        leaf
        "off"
      ]
    then
      {
        inherit leaf;
        enabled = false;
      }
    else if length f < 3 || length f > 4 then
      throw "hyprland: animation \"${spec}\" must be \"leaf, speed, kind:curve[, style]\" or \"leaf, off\""
    else
      let
        curve = lib.splitString ":" (elemAt f 2);
        kind = elemAt curve 0;
      in
      if
        length curve != 2
        || !(builtins.elem kind [
          "bezier"
          "spring"
        ])
      then
        throw "hyprland: animation \"${spec}\" must name its curve as bezier:<name> or spring:<name>"
      else
        {
          inherit leaf;
          enabled = true;
          speed = num (elemAt f 1);
          ${kind} = elemAt curve 1;
        }
        // lib.optionalAttrs (length f == 4) { style = elemAt f 3; }
  );

  # monitor = mkMonitors { "<output>" = "mode, position, scale"; … }
  # A value may instead be an attrset of raw monitor fields when extra keys
  # (vrr, transform, …) are needed; `output` is filled in from the name.
  mkMonitors = lib.mapAttrsToList (
    output: spec:
    {
      inherit output;
    }
    // (
      if lib.isAttrs spec then
        spec
      else
        let
          f = fields spec;
        in
        if length f != 3 then
          throw "hyprland: monitor \"${output}\" must be \"mode, position, scale\", got \"${spec}\""
        else
          {
            mode = elemAt f 0;
            position = elemAt f 1;
            scale = num (elemAt f 2);
          }
    )
  );

  # bind = mkBinds { "<keys>" = { rule = …; desc = …; …flags }; }
  # → hl.bind("<keys>", <rule>, { desc = …, …flags })
  mkBinds = lib.mapAttrsToList (
    keys:
    { rule, ... }@opts:
    mkArgs [
      keys
      rule
      (removeAttrs opts [ "rule" ])
    ]
  );
in
{
  inherit
    expandRules
    mkRules
    mkArgs
    mkEnv
    mkCurves
    mkAnimations
    mkMonitors
    mkBinds
    ;
}
