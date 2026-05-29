{ lib }:
let
  toList = x: if lib.isList x then x else [ x ];

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
          map (v: rule // { match = m // { ${listField} = v; }; }) m.${listField}
        )
    ) rules;

  # Build the whole window-rule list from one config:
  #   tags.<name>  = match fields → each field/value is an OR way to get +<name>
  #                  (list value = OR within field). `all` = AND-combos.
  #   apply.<name> = properties applied to windows carrying that tag.
  #   rules        = ordered list of { <label> = rule; } standalone rules.
  mkRules =
    {
      tags ? { },
      apply ? { },
      rules ? [ ],
    }:
    let
      defs = lib.concatLists (
        lib.mapAttrsToList (
          tag: spec:
          let
            fields = removeAttrs spec [ "all" ];
            combos = spec.all or [ ];
          in
          lib.concatLists (
            lib.mapAttrsToList (
              field: vals:
              map (v: {
                match.${field} = v;
                tag = "+${tag}";
              }) (toList vals)
            ) fields
          )
          ++ map (c: {
            match = c;
            tag = "+${tag}";
          }) combos
        ) tags
      );
      applies = lib.mapAttrsToList (tag: props: { match.tag = tag; } // props) apply;
    in
    expandRules (defs ++ applies ++ lib.concatMap lib.attrValues rules);
in
{
  inherit expandRules mkRules;
}
