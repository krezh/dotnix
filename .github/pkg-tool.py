#!/usr/bin/env python3
import json
import subprocess
import sys
import os
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import cast

GREEN = "\033[32m"
RED   = "\033[31m"
BLUE  = "\033[34m"
YELLOW = "\033[33m"
BOLD  = "\033[1m"
RESET = "\033[0m"

def c(txt: str, col: str, github: bool = False) -> str:
    if github:
        return str(txt)
    return f"{col}{txt}{RESET}"

def load_json(path: str) -> dict[str, object]:
    with open(path) as f:
        return cast(dict[str, object], json.load(f))

def drv_role(drv_path: str) -> str:
    return drv_path.split('-', 1)[-1]

def drv_hash(drv_path: str) -> str:
    return drv_path.split('/')[-1].split('-', 1)[0]

def get_store_paths(path: str) -> list[str]:
    proc = subprocess.run(
        ["nix-store", "--query", "--requisites", path],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True
    )
    return [line.strip() for line in proc.stdout.splitlines() if line.strip()]

def get_derivers(paths: list[str]) -> list[str]:
    proc = subprocess.run(
        ["nix-store", "--query", "--deriver"] + paths,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True
    )
    return [line.strip() for line in proc.stdout.splitlines() if line.strip() and line.strip() != "unknown"]

def get_meta_from_drv(drv_path: str) -> tuple[str, str]:
    proc = subprocess.run(
        ["nix", "derivation", "show", drv_path],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True
    )
    try:
        data: dict[str, dict[str, object]] = json.loads(proc.stdout)
    except Exception:
        return "", ""
    meta: dict[str, object] = data.get(drv_path, {})
    env: dict[str, object] = meta.get("env", {}) if isinstance(meta.get("env", {}), dict) else {}
    pname: str = str(env.get("pname", "")) if env.get("pname", "") is not None else ""
    version: str = str(env.get("version", "")) if env.get("version", "") is not None else ""
    if pname:
        if version:
            return pname, version
        base = os.path.basename(drv_path)
        hash_part = base.split('-')[0] if '-' in base else ""
        return pname, hash_part
    return "", ""

def parallel_pkg_list(derivers: list[str]) -> tuple[dict[str, str], list[str]]:
    results: list[tuple[str, str, str]] = []
    max_workers = min(32, os.cpu_count() or 4)
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_drv = {executor.submit(get_meta_from_drv, drv): drv for drv in derivers}
        for future in as_completed(future_to_drv):
            name, version = future.result()
            drv = future_to_drv[future]
            results.append((drv, name, version))
    results.sort()
    pkgs: dict[str, str] = {}
    derivations_without_pname: list[str] = []
    for drv, name, version in results:
        if name:
            if name not in pkgs:
                pkgs[name] = version
        else:
            derivations_without_pname.append(drv)
    return pkgs, derivations_without_pname

def pkg_list_main(result_path: str, output_json: str) -> None:
    store_paths = get_store_paths(result_path)
    derivers = sorted(set(get_derivers(store_paths)))
    pkgs, derivations_without_pname = parallel_pkg_list(derivers)
    output = {
        "packages": pkgs,
        "derivations_without_pname": derivations_without_pname
    }
    with open(output_json, "w") as f:
        json.dump(output, f, indent=2)
    print(f"Wrote {len(pkgs)} packages and {len(derivations_without_pname)} non-package derivations to {output_json}")

def pkg_diff_main(old_path: str, new_path: str, github: bool = False) -> None:
    old = load_json(old_path)
    new = load_json(new_path)

    old_pkgs = cast(dict[str, str], old.get("packages", {}))
    new_pkgs = cast(dict[str, str], new.get("packages", {}))
    old_keys: set[str] = set(old_pkgs.keys())
    new_keys: set[str] = set(new_pkgs.keys())

    pkg_added: list[str] = sorted(new_keys - old_keys)
    pkg_removed: list[str] = sorted(old_keys - new_keys)
    pkg_changed: list[str] = sorted(k for k in old_keys & new_keys if old_pkgs[k] != new_pkgs[k])

    old_drvs_list = cast(list[str], old.get("derivations_without_pname", []))
    new_drvs_list = cast(list[str], new.get("derivations_without_pname", []))
    old_drvs: set[str] = set(old_drvs_list)
    new_drvs: set[str] = set(new_drvs_list)
    drv_added: list[str] = sorted(new_drvs - old_drvs)
    drv_removed: list[str] = sorted(old_drvs - new_drvs)

    added_roles: dict[str, str] = {drv_role(d): d for d in drv_added}
    removed_roles: dict[str, str] = {drv_role(d): d for d in drv_removed}
    drv_changed: set[str] = set(added_roles) & set(removed_roles)
    drv_added = [added_roles[r] for r in added_roles if r not in drv_changed]
    drv_removed = [removed_roles[r] for r in removed_roles if r not in drv_changed]

    if github:
        print(f"### Package Diff\n")
        print(f"**Files:**\n")
        print(f"- `{os.path.abspath(old_path)}` (old)")
        print(f"- `{os.path.abspath(new_path)}` (new)\n")
        print(f"**Summary:** {len(pkg_added)} added, {len(pkg_removed)} removed, {len(pkg_changed)} changed\n")

        print("```diff")
        # Markdown-style header for packages
        print("# --- Packages ---")
        # Packages
        for k in pkg_added:
            print(f"+ [A+] {k} {new_pkgs[k]}")
        for k in pkg_removed:
            print(f"- [R-] {k} {old_pkgs[k]}")
        for k in pkg_changed:
            print(f"! [C*] {k} {old_pkgs[k]} -> {new_pkgs[k]}")
        # Markdown-style header for derivations
        print("# --- Derivations ---")
        # Derivations
        for r in sorted(drv_changed):
            old_hash = drv_hash(removed_roles[r])
            new_hash = drv_hash(added_roles[r])
            print(f"! [DC] {r}: {old_hash} -> {new_hash}")
        for d in drv_added:
            role = drv_role(d)
            hash_ = drv_hash(d)
            print(f"+ [DA+] {role}: {hash_}")
        for d in drv_removed:
            role = drv_role(d)
            hash_ = drv_hash(d)
            print(f"- [DR-] {role}: {hash_}")
        print("```\n")

        if not (pkg_added or pkg_removed or pkg_changed or drv_changed or drv_added or drv_removed):
            print("No changes.\n")

        print(f"Structured diff written to `pkg-diff.json`")
    else:
        print(f"<<< {os.path.abspath(old_path)}")
        print(f">>> {os.path.abspath(new_path)}\n")

        num: int = 1

        if pkg_changed:
            print("Package version changes:")
            for k in pkg_changed:
                print(f"[C*] #{num} {c(k, GREEN, github)} {c(old_pkgs[k], YELLOW, github)} -> {c(new_pkgs[k], YELLOW, github)}")
                num += 1
            print()

        if pkg_added:
            print("Added packages:")
            for k in pkg_added:
                print(f"[A+] #{num} {c(k, GREEN, github)} {c(new_pkgs[k], YELLOW, github)}")
                num += 1
            print()

        if pkg_removed:
            print("Removed packages:")
            for k in pkg_removed:
                print(f"[R-] #{num} {c(k, GREEN, github)} {c(old_pkgs[k], YELLOW, github)}")
                num += 1
            print()

        if drv_changed or drv_added or drv_removed:
            print("Derivations:")
            for r in sorted(drv_changed):
                old_hash = drv_hash(removed_roles[r])
                new_hash = drv_hash(added_roles[r])
                prefix = f"[DC] {r}:"
                print(prefix)
                print(f"{' ' * 5}{c(old_hash, RED, github)} -> {c(new_hash, GREEN, github)}")
            for d in drv_added:
                role = drv_role(d)
                hash_ = drv_hash(d)
                print(f"[DA+] {role}: {c(hash_, GREEN, github)}")
            for d in drv_removed:
                role = drv_role(d)
                hash_ = drv_hash(d)
                print(f"[DR-] {role}: {c(hash_, RED, github)}")
            print()

        if num == 1 and not (drv_changed or drv_added or drv_removed):
            print("No changes.")

        print("Structured diff written to pkg-diff.json")

    with open("pkg-diff.json", "w") as f:
        json.dump({
            "added": pkg_added,
            "removed": pkg_removed,
            "changed": [
                {"key": k, "old": old_pkgs[k], "new": new_pkgs[k]} for k in pkg_changed
            ],
            "derivations": {
                "changed": [
                    {"role": r, "old": drv_hash(removed_roles[r]), "new": drv_hash(added_roles[r])}
                    for r in sorted(drv_changed)
                ],
                "added": [
                    {"role": drv_role(d), "hash": drv_hash(d)} for d in drv_added
                ],
                "removed": [
                    {"role": drv_role(d), "hash": drv_hash(d)} for d in drv_removed
                ]
            }
        }, f, indent=2)

def main() -> None:
    if len(sys.argv) < 2:
        print("Usage:")
        print("  pkg-tool.py list <result-symlink> <output.json>")
        print("  pkg-tool.py diff <old.json> <new.json> [--github]")
        sys.exit(1)
    mode = sys.argv[1]
    if mode == "list":
        if len(sys.argv) != 4:
            print("Usage: pkg-tool.py list <result-symlink> <output.json>")
            sys.exit(1)
        pkg_list_main(sys.argv[2], sys.argv[3])
    elif mode == "diff":
        github = False
        args = sys.argv[2:]
        if "--github" in args:
            github = True
            args.remove("--github")
        if len(args) != 2:
            print("Usage: pkg-tool.py diff <old.json> <new.json> [--github]")
            sys.exit(1)
        pkg_diff_main(args[0], args[1], github)
    else:
        print("Unknown mode:", mode)
        sys.exit(1)

if __name__ == "__main__":
    main()
