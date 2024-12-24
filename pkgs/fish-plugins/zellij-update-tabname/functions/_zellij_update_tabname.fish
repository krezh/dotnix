function _zellij_update_tabname
    if set -q ZELLIJ
        set current_dir $PWD
        if test $current_dir = $HOME
            set tab_name "~"
        else
            set tab_name (basename $current_dir)
        end

        if fish_git_prompt >/dev/null
            # we are in a git repo

            # if we are in a git superproject, use the superproject name
            # otherwise, use the toplevel repo name
            set git_root (git rev-parse --show-superproject-working-tree)
            if test -z $git_root
                set git_root (git rev-parse --show-toplevel)
            end

            #  if we are in a subdirectory of the git root, use the relative path
            if test (string lower "$git_root") != (string lower "$current_dir")
                set tab_name (basename $git_root)/(basename $current_dir)
            end
        end

        nohup zellij action rename-tab $tab_name >/dev/null 2>&1
    end
end
