# starship

Starship prompt config. The `custom.jj` module is driven by [`jj-starship`](https://github.com/dmmulroy/jj-starship),
a unified git + jj prompt segment.

## Repo symbols

| Symbol | Meaning  |
| ------ | -------- |
| `󱗆`    | jj repo  |
| ``    | git repo |

## jj status

| Symbol | Meaning                                          |
| ------ | ------------------------------------------------ |
| `!`    | conflict                                         |
| `⇔`    | divergent                                        |
| `∅`    | empty description (on a non-empty commit)        |
| `⇡`    | current or closest bookmark unsynced with remote |
| `~N`   | distance to ancestor bookmark (e.g. `main~3`)    |

## git status

| Symbol | Meaning     |
| ------ | ----------- |
| `=`    | conflicted  |
| `+`    | staged      |
| `!`    | modified    |
| `?`    | untracked   |
| `✘`    | deleted     |
| `⇡N`   | ahead by N  |
| `⇣N`   | behind by N |
