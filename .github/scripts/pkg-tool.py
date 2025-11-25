#!/usr/bin/env python3
import json
import subprocess
import sys
import os
import re
from collections import defaultdict, Counter
from typing import TypedDict, cast

if sys.version_info >= (3, 12):
    from typing import override
else:
    def override(func):
        return func

debug_logging: bool = False

# SGR integer codes for nvd-style coloring
SGR_RESET: str = "\033[0m"
SGR_BOLD: str = "\033[1m"
SGR_FG: int = 30
SGR_BG: int = 40
SGR_BRIGHT: int = 60
SGR_BLACK: int = 0
SGR_RED: int = 1
SGR_GREEN: int = 2
SGR_YELLOW: int = 3
SGR_BLUE: int = 4
SGR_MAGENTA: int = 5
SGR_CYAN: int = 6
SGR_WHITE: int = 7


def sgr(*args: int) -> str:
    return "\033[" + ";".join(str(a) for a in args) + "m"


def color(txt: str, fg: int, bright: bool = False, bold: bool = False) -> str:
    codes: list[int] = [SGR_FG + fg]
    if bright:
        codes[0] += SGR_BRIGHT
    if bold:
        codes.insert(0, 1)
    return f"{sgr(*codes)}{SGR_BOLD if bold else ''}{txt}{SGR_RESET}"


def status_color(code: str) -> str:
    # nvd-style status coloring
    if code == "U":
        return color(f"[U]", SGR_CYAN, bright=True, bold=True)
    elif code == "D":
        return color(f"[D]", SGR_YELLOW, bright=True, bold=True)
    elif code == "A":
        return color(f"[A]", SGR_GREEN, bright=True, bold=True)
    elif code == "R":
        return color(f"[R]", SGR_RED, bright=True, bold=True)
    elif code == "C":
        return color(f"[C]", SGR_MAGENTA, bright=True, bold=True)
    else:
        return color(f"[{code}]", SGR_WHITE, bright=True, bold=True)


def sel_color(sel: str) -> str:
    # nvd-style selection state coloring
    if sel == "*":
        return color("*", SGR_GREEN, bright=True, bold=True)
    elif sel == ".":
        return color(".", SGR_WHITE)
    elif sel == "+":
        return color("+", SGR_GREEN, bright=True, bold=True)
    elif sel == "-":
        return color("-", SGR_RED, bright=True, bold=True)
    else:
        return color(sel, SGR_WHITE)


NIX_STORE_PATH_REGEX: re.Pattern[str] = re.compile(
    r"^/nix/store/[a-z0-9]+-(.+?)(-([0-9].*?))?(\.drv)?$"
)


class PkgVersionEntry(TypedDict):
    version: str
    selected: bool


class GroupedVersionEntry(TypedDict):
    version: str
    selected: bool
    count: int


class PackagesJson(TypedDict):
    packages: dict[str, list[PkgVersionEntry]]
    derivations_without_pname: list[str]


class Version:
    def __init__(self, version_str: str) -> None:
        self.original: str = version_str
        self.chunks: list[int | str] = self._parse(version_str)

    def _parse(self, s: str) -> list[int | str]:
        # Split into numeric and non-numeric chunks
        chunks: list[str] = re.findall(r"\d+|[a-zA-Z]+|[^a-zA-Z\d]+", s)
        return [int(x) if x.isdigit() else x for x in chunks]

    @override
    def __eq__(self, other: object) -> bool:
        return isinstance(other, Version) and self.chunks == other.chunks

    def __lt__(self, other: object) -> bool:
        if not isinstance(other, Version):
            return NotImplemented
        for a, b in zip(self.chunks, other.chunks):
            if a == b:
                continue
            if isinstance(a, int) and isinstance(b, int):
                return a < b
            if isinstance(a, int):
                return True
            if isinstance(b, int):
                return False
            return str(a) < str(b)
        return len(self.chunks) < len(other.chunks)

    def __gt__(self, other: object) -> bool:
        return not (self == other or self < other)

    @override
    def __repr__(self) -> str:
        return f"Version({self.original})"


def load_json(path: str) -> PackagesJson:
    with open(path) as f:
        return cast(PackagesJson, json.load(f))


def get_store_paths(path: str) -> list[str]:
    proc: subprocess.CompletedProcess[str] = subprocess.run(
        ["nix-store", "--query", "--requisites", path],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    return [line.strip() for line in proc.stdout.splitlines() if line.strip()]


def get_selected_paths(path: str) -> list[str]:
    proc: subprocess.CompletedProcess[str] = subprocess.run(
        ["nix-store", "--query", "--references", path],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    return [line.strip() for line in proc.stdout.splitlines() if line.strip()]


def parse_pname_version(store_path: str) -> tuple[str, str]:
    match: re.Match[str] | None = NIX_STORE_PATH_REGEX.match(store_path)
    if not match:
        return "", ""
    pname: str = match.group(1) or ""
    version: str = match.group(3) or ""
    return pname, version


def pkg_list_main(result_path: str, output_json: str) -> None:
    closure_paths: list[str] = get_store_paths(result_path)
    selected_paths: set[str] = set(get_selected_paths(result_path))

    pkgs: dict[str, list[PkgVersionEntry]] = defaultdict(list)
    derivations_without_pname: list[str] = []

    for path in closure_paths:
        pname, version = parse_pname_version(path)
        selected: bool = path in selected_paths
        if pname:
            pkgs[pname].append({"version": version, "selected": selected})
        else:
            derivations_without_pname.append(path)

    output: PackagesJson = {
        "packages": pkgs,
        "derivations_without_pname": derivations_without_pname,
    }
    with open(output_json, "w") as f:
        json.dump(output, f, indent=2)


def group_versions(pkg_versions: list[PkgVersionEntry]) -> list[GroupedVersionEntry]:
    # Returns a sorted list of (version, count, selected) using semantic version sorting
    counter: Counter[tuple[str, bool]] = Counter(
        (v["version"], v["selected"])
        for v in pkg_versions
        if "version" in v and "selected" in v
    )

    def version_key(item: tuple[tuple[str, bool], int]) -> tuple[Version | str, bool]:
        version, selected = item[0]
        try:
            return (Version(version), selected)
        except Exception:
            return (version, selected)

    result: list[GroupedVersionEntry] = []
    for (version, selected), count in sorted(counter.items(), key=version_key):
        result.append({"version": version, "selected": selected, "count": count})
    return result


def pkg_diff_main(
    old_path: str,
    new_path: str,
    github: bool = False,
    diff_output_path: str = "pkg-diff.json",
) -> None:
    old: PackagesJson = load_json(old_path)
    new: PackagesJson = load_json(new_path)

    old_pkgs: dict[str, list[PkgVersionEntry]] = old.get("packages", {})
    new_pkgs: dict[str, list[PkgVersionEntry]] = new.get("packages", {})
    all_keys: set[str] = set(old_pkgs.keys()) | set(new_pkgs.keys())

    pkg_added: list[tuple[str, list[GroupedVersionEntry]]] = []
    pkg_removed: list[tuple[str, list[GroupedVersionEntry]]] = []
    pkg_changed: list[
        tuple[str, list[GroupedVersionEntry], list[GroupedVersionEntry], str]
    ] = []
    pkg_sel_changed: list[
        tuple[str, list[GroupedVersionEntry], list[GroupedVersionEntry]]
    ] = []

    for k in sorted(all_keys):
        old_versions: list[GroupedVersionEntry] = group_versions(old_pkgs.get(k, []))
        new_versions: list[GroupedVersionEntry] = group_versions(new_pkgs.get(k, []))

        old_set: set[tuple[str, bool, int]] = {
            (v["version"], v["selected"], v["count"]) for v in old_versions
        }
        new_set: set[tuple[str, bool, int]] = {
            (v["version"], v["selected"], v["count"]) for v in new_versions
        }

        if not old_versions and new_versions:
            pkg_added.append((k, new_versions))
        elif old_versions and not new_versions:
            pkg_removed.append((k, old_versions))
        elif old_set != new_set:
            old_versions_sorted: list[GroupedVersionEntry] = sorted(
                [v for v in old_versions if v["version"]],
                key=lambda v: Version(v["version"]),
                reverse=True,
            )
            new_versions_sorted: list[GroupedVersionEntry] = sorted(
                [v for v in new_versions if v["version"]],
                key=lambda v: Version(v["version"]),
                reverse=True,
            )
            change_code: str = "[C*]"
            if old_versions_sorted and new_versions_sorted:
                old_ver: Version = Version(old_versions_sorted[0]["version"])
                new_ver: Version = Version(new_versions_sorted[0]["version"])
                if new_ver > old_ver:
                    change_code = "[C+]"  # Upgrade
                elif new_ver < old_ver:
                    change_code = "[C-]"  # Downgrade
            pkg_changed.append((k, old_versions, new_versions, change_code))
        else:
            old_sel: list[tuple[str, bool]] = sorted(
                (v["version"], v["selected"]) for v in old_versions
            )
            new_sel: list[tuple[str, bool]] = sorted(
                (v["version"], v["selected"]) for v in new_versions
            )
            if old_sel != new_sel:
                pkg_sel_changed.append((k, old_versions, new_versions))

    def render_versions(
        version_list: list[GroupedVersionEntry], use_color: bool = True
    ) -> str:
        items: list[str] = []
        for v in version_list:
            text: str = v["version"] if v["version"] else "<none>"
            count: str = f" x{v['count']}" if v["count"] > 1 else ""
            if text == "<none>":
                items.append(f"{text}{count}")
            else:
                if use_color:
                    items.append(f"{color(text, SGR_YELLOW)}{count}")
                else:
                    items.append(f"{text}{count}")
        return ", ".join(items)

    if github:
        print(f"### Package Diff\n")
        print(f"**Files:**\n")
        print(f"- `{os.path.abspath(old_path)}` (old)")
        print(f"- `{os.path.abspath(new_path)}` (new)\n")
        print(
            f"**Summary:** {len(pkg_added)} added, {len(pkg_removed)} removed, {len(pkg_changed)} changed\n"
        )

        print("```diff")
        if pkg_added:
            print("# --- Added Packages ---")
            for k, new_versions in pkg_added:
                print(f"+ [A+] {k} {render_versions(new_versions, use_color=False)}")
        if pkg_changed:
            print("# --- Changed Packages ---")
            for k, old_versions, new_versions, change_code in pkg_changed:
                print(
                    f"! {change_code} {k} {render_versions(old_versions, use_color=False)} -> {render_versions(new_versions, use_color=False)}"
                )
        if pkg_removed:
            print("# --- Removed Packages ---")
            for k, old_versions in pkg_removed:
                print(f"- [R-] {k} {render_versions(old_versions, use_color=False)}")
        if pkg_sel_changed:
            print("# --- Selection State Changes ---")
            for k, old_versions, new_versions in pkg_sel_changed:
                print(
                    f"! [S*] {k} selection changed {render_versions(old_versions, use_color=False)} -> {render_versions(new_versions, use_color=False)}"
                )
        print("```\n")

        if not (pkg_added or pkg_removed or pkg_changed or pkg_sel_changed):
            print("No changes.\n")

    else:
        print(f"<<< {os.path.abspath(old_path)}")
        print(f">>> {os.path.abspath(new_path)}\n")

        # Calculate column widths for alignment
        pkg_name_width: int = (
            max(
                [len(k) for k, *_ in pkg_changed]
                + [len(k) for k, *_ in pkg_sel_changed]
                + [len(k) for k, *_ in pkg_added]
                + [len(k) for k, *_ in pkg_removed]
            )
            if (pkg_changed or pkg_sel_changed or pkg_added or pkg_removed)
            else 30
        )

        num: int = 1
        if pkg_changed:
            print("Version changes:")
            count_width: int = len(str(len(pkg_changed)))
            count_format_str: str = "#{:0" + str(count_width) + "d}"
            pkg_name_format_str: str = "{:" + str(pkg_name_width) + "}"
            for k, old_versions, new_versions, change_code in sorted(
                pkg_changed, key=lambda x: x[0].lower()
            ):
                old_versions_sorted = sorted(
                    old_versions,
                    key=lambda v: Version(v["version"])
                    if v["version"]
                    else Version("0"),
                    reverse=True,
                )
                new_versions_sorted = sorted(
                    new_versions,
                    key=lambda v: Version(v["version"])
                    if v["version"]
                    else Version("0"),
                    reverse=True,
                )
                if change_code == "[C+]":
                    status: str = "U"
                elif change_code == "[C-]":
                    status = "D"
                elif change_code == "[C*]":
                    status = "C"
                else:
                    status = "C"
                sel: str = (
                    "*" if any(v["selected"] for v in new_versions_sorted) else "."
                )
                status_str: str = status_color(status) + sel_color(sel)
                count_str: str = count_format_str.format(num)
                pkg_str: str = color(
                    pkg_name_format_str.format(k), SGR_GREEN, bright=True, bold=True
                )
                old_ver_str: str = render_versions(old_versions_sorted, use_color=True)
                new_ver_str: str = render_versions(new_versions_sorted, use_color=True)
                print(
                    f"{status_str}  {count_str}  {pkg_str}  {old_ver_str} -> {new_ver_str}"
                )
                num += 1
            print()

        # Selection state changes
        if pkg_sel_changed:
            print("Selection state changes:")
            count_width = len(str(len(pkg_sel_changed)))
            count_format_str = "#{:0" + str(count_width) + "d}"
            pkg_name_format_str = "{:" + str(pkg_name_width) + "}"
            num = 1
            for k, old_versions, new_versions in sorted(
                pkg_sel_changed, key=lambda x: x[0].lower()
            ):
                old_versions_sorted = sorted(
                    old_versions,
                    key=lambda v: Version(v["version"])
                    if v["version"]
                    else Version("0"),
                    reverse=True,
                )
                new_versions_sorted = sorted(
                    new_versions,
                    key=lambda v: Version(v["version"])
                    if v["version"]
                    else Version("0"),
                    reverse=True,
                )
                status_str = status_color("C") + sel_color(
                    "*" if any(v["selected"] for v in new_versions_sorted) else "."
                )
                count_str = count_format_str.format(num)
                pkg_str = color(
                    pkg_name_format_str.format(k), SGR_GREEN, bright=True, bold=True
                )
                old_ver_str = render_versions(old_versions_sorted, use_color=True)
                new_ver_str = render_versions(new_versions_sorted, use_color=True)
                print(
                    f"{status_str}  {count_str}  {pkg_str}  {old_ver_str} -> {new_ver_str}"
                )
                num += 1
            print()

        # Added packages
        if pkg_added:
            print("Added packages:")
            count_width = len(str(len(pkg_added)))
            count_format_str = "#{:0" + str(count_width) + "d}"
            pkg_name_format_str = "{:" + str(pkg_name_width) + "}"
            num = 1
            for k, new_versions in sorted(pkg_added, key=lambda x: x[0].lower()):
                new_versions_sorted = sorted(
                    new_versions,
                    key=lambda v: Version(v["version"])
                    if v["version"]
                    else Version("0"),
                    reverse=True,
                )
                status_str = status_color("A") + sel_color(
                    "*" if any(v["selected"] for v in new_versions_sorted) else "."
                )
                count_str = count_format_str.format(num)
                pkg_str = color(
                    pkg_name_format_str.format(k), SGR_GREEN, bright=True, bold=True
                )
                ver_str: str = render_versions(new_versions_sorted, use_color=True)
                print(f"{status_str}  {count_str}  {pkg_str}  {ver_str}")
                num += 1
            print()

        # Removed packages
        if pkg_removed:
            print("Removed packages:")
            count_width = len(str(len(pkg_removed)))
            count_format_str = "#{:0" + str(count_width) + "d}"
            pkg_name_format_str = "{:" + str(pkg_name_width) + "}"
            num = 1
            for k, old_versions in sorted(pkg_removed, key=lambda x: x[0].lower()):
                old_versions_sorted = sorted(
                    old_versions,
                    key=lambda v: Version(v["version"])
                    if v["version"]
                    else Version("0"),
                    reverse=True,
                )
                status_str = status_color("R") + sel_color(
                    "*" if any(v["selected"] for v in old_versions_sorted) else "."
                )
                count_str = count_format_str.format(num)
                pkg_str = color(
                    pkg_name_format_str.format(k), SGR_GREEN, bright=True, bold=True
                )
                ver_str = render_versions(old_versions_sorted, use_color=True)
                print(f"{status_str}  {count_str}  {pkg_str}  {ver_str}")
                num += 1
            print()

        if num == 1:
            print("No changes.")

    # Write structured diff to JSON (grouped outputs)
    with open(diff_output_path, "w") as f:
        json.dump(
            {
                "added": [
                    {"key": k, "versions": new_versions}
                    for k, new_versions in pkg_added
                ],
                "removed": [
                    {"key": k, "versions": old_versions}
                    for k, old_versions in pkg_removed
                ],
                "changed": [
                    {
                        "key": k,
                        "old": old_versions,
                        "new": new_versions,
                        "code": change_code,
                    }
                    for k, old_versions, new_versions, change_code in pkg_changed
                ],
                "selection_changed": [
                    {"key": k, "old": old_versions, "new": new_versions}
                    for k, old_versions, new_versions in pkg_sel_changed
                ],
            },
            f,
            indent=2,
        )


def main() -> None:
    global debug_logging
    if len(sys.argv) < 2:
        print("Usage:")
        print("  pkg-tool.py list <result-symlink> <output.json> [--debug]")
        print(
            "  pkg-tool.py diff <old.json> <new.json> [--github] [--debug] [--out <diff.json>]"
        )
        sys.exit(1)
    mode: str = sys.argv[1]
    args: list[str] = sys.argv[2:]
    if "--debug" in args:
        debug_logging = True
        args.remove("--debug")
    diff_output_path = "pkg-diff.json"
    if mode == "list":
        if len(args) != 2:
            print("Usage: pkg-tool.py list <result-symlink> <output.json> [--debug]")
            sys.exit(1)
        pkg_list_main(args[0], args[1])
    elif mode == "diff":
        github: bool = False
        if "--github" in args:
            github = True
            args.remove("--github")
        if "--out" in args:
            out_idx = args.index("--out")
            if out_idx + 1 >= len(args):
                print(
                    "Usage: pkg-tool.py diff <old.json> <new.json> [--github] [--debug] [--out <diff.json>]"
                )
                sys.exit(1)
            diff_output_path = args[out_idx + 1]
            del args[out_idx : out_idx + 2]
        if len(args) != 2:
            print(
                "Usage: pkg-tool.py diff <old.json> <new.json> [--github] [--debug] [--out <diff.json>]"
            )
            sys.exit(1)
        pkg_diff_main(args[0], args[1], github, diff_output_path)
    else:
        print("Unknown mode:", mode)
        sys.exit(1)


if __name__ == "__main__":
    main()
