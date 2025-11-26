Awesome. Let's get your hands dirty. We are going to set up a professional Go Project Layout. This structure is designed to scale so that when you add the "Context Engine" and "Firebase" logic later, you won't end up with one massive, unreadable main.go file.
Step 1: Create the Directory Structure
Open your terminal (or VS Code terminal) and run these commands to create the project skeleton. We are separating the Core Logic (Internal) from the Entry Point (Cmd).
# 1. Create the project folder
mkdir c500-timesync
cd c500-timesync

# 2. Initialize the Go Module (Replace 'yourusername' with your actual GitHub handle)
go mod init github.com/yourusername/c500-timesync

# 3. Create the folder structure
mkdir -p cmd/bot
mkdir -p internal/discord
mkdir -p internal/firestore
mkdir -p internal/context
mkdir -p internal/config

Step 2: Install Dependencies
We need to pull in the libraries we discussed in the architecture phase. Run these commands:
# The Discord API Wrapper
go get github.com/bwmarrin/discordgo

# The Google Cloud / Firebase SDK
go get firebase.google.com/go
go get google.golang.org/api/option

# The Astronomy Calculator (for Sunrise/Sunset)
go get github.com/nathan-osman/go-sunrise

# (Optional but recommended) Dotenv for managing API keys locally
go get github.com/joho/godotenv

Step 3: Create the Entry Point (main.go)
Create a file at cmd/bot/main.go.
This will be the "Brain" of your application. For now, we will just set it up to verify that it runs and prints "System Starting..."
package main

import (
	"log"
	"os"

	"github.com/joho/godotenv"
)

func main() {
	// 1. Load Environment Variables (API Keys, etc.)
	err := godotenv.Load()
	if err != nil {
		log.Println("No .env file found, relying on system env vars")
	}

	log.Println("=== C500 TimeSync System Starting ===")

	// 2. Check for required keys (We will add these next)
	token := os.Getenv("DISCORD_BOT_TOKEN")
	if token == "" {
		log.Fatal("Error: DISCORD_BOT_TOKEN is not set")
	}

	log.Println("✅ Environment loaded. Ready to connect.")
	
	// TODO: Initialize Firestore
	// TODO: Initialize Discord Session
	// TODO: Start Cloud Scheduler Listener
}

Step 4: Create your .env file
In the root of your project (same level as go.mod), create a file named .env.
Important: Add this file to your .gitignore immediately so you don't accidentally publish your keys.
# .env file
DISCORD_BOT_TOKEN=your_token_goes_here
GOOGLE_APPLICATION_CREDENTIALS=./service-account.json
FIRESTORE_PROJECT_ID=your_project_id

Checkpoint:
Run the project to make sure the environment is sane:
go run cmd/bot/main.go
It should panic (crash) because you don't have a token yet. That means it works!
What is your next move?
 * Get a Discord Token: Go to the Discord Developer Portal and create the App/Bot.
 * Get a Firebase Key: Go to the Firebase Console and generate a Service Account JSON.
Which one do you want to tackle first?
