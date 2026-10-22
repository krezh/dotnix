//! `island completions SHELL`: a script for each shell that asks `island __complete` what can
//! come next, so completions follow the same descriptions as the help and never go stale.

use super::{
    mistake::{Mistake, listed},
    paint::{Line, Tone},
    program::SHELLS,
};

const FISH: &str = r#"function __island_complete
    set -l words (commandline -opc)
    set -e words[1]
    set -l out (ISLAND __complete -- $words (commandline -ct))
    if test "$out[1]" = "<files>"
        __fish_complete_path (commandline -ct)
    else
        printf '%s\n' $out
    end
end
complete -c island -f -a '(__island_complete)'
"#;

const BASH: &str = r#"_island() {
    local IFS=$'\n' current="${COMP_WORDS[COMP_CWORD]}"
    local -a out
    out=($(ISLAND __complete -- "${COMP_WORDS[@]:1:COMP_CWORD-1}" "$current"))
    if [[ "${out[0]}" == "<files>" ]]; then
        COMPREPLY=($(compgen -f -- "$current"))
    else
        COMPREPLY=("${out[@]%%$'\t'*}")
    fi
}
complete -F _island island
"#;

const ZSH: &str = r#"#compdef island
_island() {
    local -a out described
    local line
    out=("${(@f)$(ISLAND __complete -- "${(@)words[2,CURRENT-1]}" "${words[CURRENT]}")}")
    if [[ "${out[1]}" == "<files>" ]]; then
        _files
        return
    fi
    for line in "${out[@]}"; do
        [[ -n "$line" ]] && described+=("${${line%%$'\t'*}//:/\\:}:${line#*$'\t'}")
    done
    _describe island described
}
if [[ "$funcstack[1]" == "_island" ]]; then
    _island "$@"
else
    compdef _island island
fi
"#;

/// The script for the one shell in `args`.
pub fn script(args: &[String]) -> Result<String, Mistake> {
    let shells = || listed("Shells", SHELLS);
    let template = match args {
        [shell] => match shell.as_str() {
            "bash" => BASH,
            "fish" => FISH,
            "zsh" => ZSH,
            other => {
                return Err(
                    Mistake::new(Line::new().plain("No shell ").push(Tone::Strong, other))
                        .did_you_mean(other, SHELLS.iter().copied())
                        .hint(shells()),
                );
            }
        },
        [] => {
            return Err(Mistake::new(
                Line::new()
                    .push(Tone::Command, "island completions")
                    .plain(" needs a shell"),
            )
            .hint(shells()));
        }
        [_, extra @ ..] => {
            return Err(Mistake::new(
                Line::new()
                    .plain("Nothing goes after the shell, got ")
                    .push(Tone::Strong, extra.join(" ")),
            ));
        }
    };
    // This very binary, so completing never finds another island on the way.
    let island = std::env::current_exe()
        .ok()
        .and_then(|path| path.to_str().map(str::to_owned))
        .unwrap_or_else(|| "island".to_owned());
    Ok(template.replace("ISLAND", &format!("'{}'", island.replace('\'', r"'\''"))))
}

#[cfg(test)]
#[path = "completions_test.rs"]
mod tests;
