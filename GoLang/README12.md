Excellent. Let's get the Discord Bot Token and bring the "C500 Concierge" to life.
Here is the step-by-step to get your credentials and connect them to your Go code.
Phase 1: Create the Bot in the Developer Portal
 * Go to the Portal: Navigate to the Discord Developer Portal.
 * New App: Click "New Application" (Top Right).
   * Name: C500 Concierge (or C500 TimeSync).
   * Agreed to terms -> Create.
 * Create the Bot User:
   * On the left sidebar, click Bot.
   * Click "Reset Token" (Yes, do it).
   * COPY THIS TOKEN IMMEDIATELY. You will not see it again.
   * Paste it into your .env file: DISCORD_BOT_TOKEN=your_pasted_token_here
 * Enable Intents (Crucial for DMs):
   * Scroll down on the Bot page to "Privileged Gateway Intents".
   * Toggle ON: PRESENCE INTENT, SERVER MEMBERS INTENT, and MESSAGE CONTENT INTENT.
   * Why? We need MESSAGE CONTENT to read DMs if we expand beyond slash commands, and MEMBERS to look up timezones later.
   * Click "Save Changes".
Phase 2: Invite the Bot to your Server
The bot exists, but it's not in your server yet.
 * On the left sidebar, click OAuth2 -> URL Generator.
 * Scopes: Check bot and applications.commands.
 * Bot Permissions: Check Send Messages, Embed Links, Read Message History.
 * Copy the Generated URL at the bottom.
 * Paste that URL into your browser, select your server, and click Authorize.
Phase 3: Update main.go to Connect
Now we update your Go code to actually use that token. We will verify the connection by printing the bot's username when it logs in.
Open cmd/bot/main.go and replace the previous code with this:
package main

import (
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/bwmarrin/discordgo"
	"github.com/joho/godotenv"
)

func main() {
	// 1. Load Environment
	err := godotenv.Load()
	if err != nil {
		log.Println("⚠️  No .env file found. Ensure DISCORD_BOT_TOKEN is set in system environment.")
	}

	token := os.Getenv("DISCORD_BOT_TOKEN")
	if token == "" {
		log.Fatal("❌ Error: DISCORD_BOT_TOKEN is missing.")
	}

	// 2. Create Discord Session
	dg, err := discordgo.New("Bot " + token)
	if err != nil {
		log.Fatalf("❌ Error creating Discord session: %v", err)
	}

	// 3. Register the "Ready" Handler
	// This function triggers once when the bot successfully connects
	dg.AddHandler(func(s *discordgo.Session, r *discordgo.Ready) {
		log.Printf("✅ Logged in as: %s#%s", s.State.User.Username, s.State.User.Discriminator)
		log.Println("🚀 C500 TimeSync is listening...")
	})

	// 4. Set Intents (Must match what you enabled in the Portal)
	dg.Identify.Intents = discordgo.IntentsGuilds | discordgo.IntentsGuildMessages | discordgo.IntentsDirectMessages

	// 5. Open Connection
	err = dg.Open()
	if err != nil {
		log.Fatalf("❌ Error opening connection: %v", err)
	}

	// 6. Keep the Bot Running
	// This code waits for a CTRL+C (SIGTERM) to shut down gracefully
	log.Println("Press CTRL-C to exit.")
	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt)
	<-sc

	// 7. Cleanup
	dg.Close()
	log.Println("👋 System Shutting Down.")
}

Phase 4: Test It
 * Open your terminal in the project root.
 * Run: go run cmd/bot/main.go
 * Success: You should see:
   ✅ Logged in as: C500 Concierge#1234
🚀 C500 TimeSync is listening...

 * Check Discord: You should see the bot's status indicator turn Green (Online) in your server member list.
Did the bot come online successfully?
If yes, we can move to Step 2: Getting the Firebase Key so we can start building the database connection.
