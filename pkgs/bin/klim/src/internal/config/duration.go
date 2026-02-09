package config

import (
	"fmt"
	"regexp"
	"strconv"
	"time"
)

// ParseDuration parses duration strings with support for weeks (w) and days (d).
// Examples: "7d", "2w", "1w3d", "24h", "168h"
func ParseDuration(s string) (time.Duration, error) {
	// Match patterns like: 2w, 7d, 1w3d, 24h, etc.
	re := regexp.MustCompile(`(\d+)([wdhms])`)
	matches := re.FindAllStringSubmatch(s, -1)

	if len(matches) == 0 {
		// Try standard Go duration parsing
		return time.ParseDuration(s)
	}

	var total time.Duration

	for _, match := range matches {
		if len(match) != 3 {
			continue
		}

		value, err := strconv.ParseInt(match[1], 10, 64)
		if err != nil {
			return 0, fmt.Errorf("invalid duration value: %s", match[1])
		}

		unit := match[2]
		switch unit {
		case "w":
			total += time.Duration(value) * 7 * 24 * time.Hour
		case "d":
			total += time.Duration(value) * 24 * time.Hour
		case "h":
			total += time.Duration(value) * time.Hour
		case "m":
			total += time.Duration(value) * time.Minute
		case "s":
			total += time.Duration(value) * time.Second
		default:
			return 0, fmt.Errorf("unknown duration unit: %s", unit)
		}
	}

	if total == 0 {
		return 0, fmt.Errorf("invalid duration: %s", s)
	}

	return total, nil
}
