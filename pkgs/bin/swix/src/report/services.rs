//! The services a switch would touch, parsed from `switch-to-configuration dry-activate` output.

use crate::ui::theme;

/// What `switch-to-configuration dry-activate` says it would do to a unit.
///
/// Declaration order is display order and indexes `ALL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ServiceAction {
    Restart,
    Start,
    Stop,
    Reload,
    /// Changed units the switch deliberately keeps running (`would NOT stop …`).
    LeftRunning,
}

impl ServiceAction {
    pub(crate) const ALL: [Self; 5] = [
        Self::Restart,
        Self::Start,
        Self::Stop,
        Self::Reload,
        Self::LeftRunning,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Restart => "Restart",
            Self::Start => "Start",
            Self::Stop => "Stop",
            Self::Reload => "Reload",
            Self::LeftRunning => "Left running",
        }
    }

    /// The tone class this action's header and unit chips are drawn in.
    pub(crate) fn tone(self) -> &'static str {
        theme::tone(match self {
            Self::Restart => theme::YELLOW,
            Self::Start => theme::GREEN,
            Self::Stop => theme::RED,
            Self::Reload => theme::BLUE,
            Self::LeftRunning => theme::OVERLAY1,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceUnit {
    pub(crate) name: String,
    pub(crate) user: bool,
}

impl ServiceUnit {
    pub(crate) fn display_name(&self) -> &str {
        self.name.strip_suffix(".service").unwrap_or(&self.name)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ServiceGroup {
    pub(crate) action: ServiceAction,
    pub(crate) units: Vec<ServiceUnit>,
}

/// Groups dry-activate output by action, in `ServiceAction::ALL` order, omitting empty groups.
/// Lines that name no units (activation script output, notices) are ignored.
pub(crate) fn service_impact(lines: &[String]) -> Vec<ServiceGroup> {
    let mut groups = ServiceAction::ALL.map(|action| ServiceGroup {
        action,
        units: Vec::new(),
    });
    for line in lines {
        let Some((action, units)) = service_line(line) else {
            continue;
        };
        let group = &mut groups[action as usize];
        for unit in units {
            if !group.units.contains(&unit) {
                group.units.push(unit);
            }
        }
    }
    groups
        .into_iter()
        .filter(|group| !group.units.is_empty())
        .collect()
}

fn service_line(line: &str) -> Option<(ServiceAction, Vec<ServiceUnit>)> {
    let rest = line.trim().strip_prefix("would ")?;
    if rest.trim_end_matches('.') == "restart systemd" {
        let unit = ServiceUnit {
            name: "systemd".to_owned(),
            user: false,
        };
        return Some((ServiceAction::Restart, vec![unit]));
    }
    let (action, rest) = [
        ("NOT stop ", ServiceAction::LeftRunning),
        ("stop ", ServiceAction::Stop),
        ("restart ", ServiceAction::Restart),
        ("reload ", ServiceAction::Reload),
        ("start ", ServiceAction::Start),
    ]
    .into_iter()
    .find_map(|(prefix, action)| rest.strip_prefix(prefix).map(|rest| (action, rest)))?;
    let (kind, units) = rest.strip_prefix("the following ")?.split_once(':')?;
    let mut words = kind.split_whitespace();
    if !words.clone().any(|word| word == "units") {
        return None;
    }
    let user = words.any(|word| word == "user");
    let units = units
        .split(',')
        .map(str::trim)
        .filter(|unit| !unit.is_empty())
        .map(|unit| ServiceUnit {
            name: unit.to_owned(),
            user,
        })
        .collect::<Vec<_>>();
    (!units.is_empty()).then_some((action, units))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    fn names(group: &ServiceGroup) -> Vec<&str> {
        group.units.iter().map(|unit| unit.name.as_str()).collect()
    }

    #[test]
    fn groups_dry_activate_units_by_action() {
        let groups = service_impact(&lines(
            "would stop the following units: accounts-daemon.service, polkit.service\n\
             would NOT stop the following changed units: swix@0-1.service\n\
             would reload the following units: dbus-broker.service\n\
             would restart the following units: home-manager-krezh.service\n\
             would start the following units: accounts-daemon.service, polkit.service",
        ));
        let actions = groups.iter().map(|group| group.action).collect::<Vec<_>>();
        assert_eq!(
            actions,
            [
                ServiceAction::Restart,
                ServiceAction::Start,
                ServiceAction::Stop,
                ServiceAction::Reload,
                ServiceAction::LeftRunning,
            ]
        );
        assert_eq!(names(&groups[0]), ["home-manager-krezh.service"]);
        assert_eq!(
            names(&groups[1]),
            ["accounts-daemon.service", "polkit.service"]
        );
        assert_eq!(names(&groups[4]), ["swix@0-1.service"]);
        assert_eq!(groups[3].units[0].display_name(), "dbus-broker");
    }

    #[test]
    fn merges_repeated_actions_without_duplicates() {
        let groups = service_impact(&lines(
            "would restart the following units: sshd.service, tailscaled.service\n\
             would restart systemd\n\
             would restart the following units: sshd.service, nginx.service",
        ));
        assert_eq!(groups.len(), 1);
        assert_eq!(
            names(&groups[0]),
            [
                "sshd.service",
                "tailscaled.service",
                "systemd",
                "nginx.service"
            ]
        );
    }

    #[test]
    fn marks_user_units() {
        let groups = service_impact(&lines(
            "would restart the following user units: pipewire.service\n\
             would restart the following units: sshd.service",
        ));
        assert_eq!(
            groups[0].units,
            [
                ServiceUnit {
                    name: "pipewire.service".to_owned(),
                    user: true,
                },
                ServiceUnit {
                    name: "sshd.service".to_owned(),
                    user: false,
                },
            ]
        );
    }

    #[test]
    fn ignores_lines_without_units() {
        let groups = service_impact(&lines(
            "Not checking switch inhibitors (action = dry-activate)\n\
             would activate the configuration...\n\
             sops-install-secrets: Imported /etc/ssh/ssh_host_ed25519_key as age key\n\
             would start the following units:\n\
             would reload the following things: a.service",
        ));
        assert!(groups.is_empty());
        assert_eq!(
            service_line("would start the following units: socket.socket"),
            Some((
                ServiceAction::Start,
                vec![ServiceUnit {
                    name: "socket.socket".to_owned(),
                    user: false,
                }]
            ))
        );
    }
}
