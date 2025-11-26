Yes, that is a great pivot. Replacing Dubai with Riyadh/Bahrain keeps us in the Middle East time block (UTC+3), and adding Bangkok (UTC+7) bridges the massive gap between the Middle East and Australia.
This gives you a much smoother "Time Gradient" across the globe.
1. The Updated C500 Reference Map
Here are the new IANA codes for your GoLang backend.

| Friendly Name | IANA Identifier | Notes |
|---|---|---|
| Phoenix | America/Phoenix | UTC-7 (No DST) |
| Honolulu | Pacific/Honolulu | UTC-10 (No DST) |
| Sweden | Europe/Stockholm | UTC+1/UTC+2 (Observes DST) |
| Riyadh / Bahrain | Asia/Riyadh | UTC+3 (No DST). Note: Bahrain (Asia/Bahrain) shares the exact same time rules. |
| Bangkok | Asia/Bangkok | UTC+7 (No DST). A stable anchor for SE Asia. |
| Sydney | Australia/Sydney | UTC+10/UTC+11 (Observes DST). |

2. The Updated GoLang Logic
Here is the revised code block. I have updated the c500Locations map to reflect your new "Time Anchors."

```
package main

import (
	"fmt"
	"time"
)

func main() {
	// 1. The Meeting Time (e.g., Nov 27th at 3:00 PM UTC)
	meetingTimeUTC := time.Date(2025, 11, 27, 15, 0, 0, 0, time.UTC)

	// 2. Updated C500 Locations
	c500Locations := []struct {
		Label string
		Code  string
	}{
		// Using a slice/struct instead of a map to keep the print order logical (West -> East)
		{"Honolulu (Ohana)", "Pacific/Honolulu"},
		{"Phoenix (HQ)", "America/Phoenix"},
		{"Sweden (Student)", "Europe/Stockholm"},
		{"Riyadh/Bahrain", "Asia/Riyadh"}, // Represents the Middle East block
		{"Bangkok (Asia)", "Asia/Bangkok"}, // New SE Asia Anchor
		{"Sydney (Student)", "Australia/Sydney"},
	}

	fmt.Println("=== C500 GLOBAL CLASS SCHEDULE ===")
	fmt.Printf("Original Time (UTC): %s\n\n", meetingTimeUTC.Format("15:04 Mon, Jan 02"))
```

	// 3. The Converter Loop

  ```
	for _, locData := range c500Locations {
		loc, err := time.LoadLocation(locData.Code)
		if err != nil {
			fmt.Printf("Error loading %s: %v\n", locData.Label, err)
			continue
		}

		localTime := meetingTimeUTC.In(loc)

		// Format output to align columns nicely
		// %-20s adds padding to the name so the times line up
		fmt.Printf("%-20s -> %s\n", locData.Label, localTime.Format("3:04 PM Mon (MST)"))
	}
}
```

Why this specific mix is better
By adding Bangkok, you have solved a hidden problem: The 7-hour Gap.
 * Previously: You had a jump from Dubai (UTC+4) to Sydney (UTC+10/11). That is a 6-7 hour blind spot where many Asian students live.
 * Now: Bangkok (UTC+7) sits perfectly in the middle.
Next Step: Database Structure
Now that we have the locations locked down, we need to decide how to store the "Availability Slots" in Firebase so they can be queried efficiently.
Which of these two options fits your teaching style better?
 * Option A: The "Open Blocks" Style. You say "I am free 9am-5pm on Monday," and the system automatically chops that up into 30-minute chunks for students to grab. (Easier for you to manage).
 * Option B: The "Specific Slots" Style. You manually create specific 30-minute blocks like "Monday 9:00", "Monday 9:30", "Monday 2:00". (Gives you total control over breaks).
