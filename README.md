# Sentinel

Sentinel is a desktop AI assistant for Windows.  
It reads the current screen context and answers your question in chat.

## What Sentinel Does

1. You ask a question in the floating chat window.
2. Sentinel captures the current foreground app window.
3. Sentinel sends your question + screenshot context to the selected AI model.
4. Sentinel returns:
   - `screen_summary`
   - `answer`
   - `suggested_next_steps`
   - `questions_to_clarify`
   - `confidence`

## Install

Install Sentinel from the latest release asset:

- `.exe` setup installer (recommended)
- `.msi` installer
- `.zip` portable package

After installation, launch Sentinel from Start Menu (or run `sentinel.exe` for portable mode).

## First-Time Setup

1. Open Sentinel.
2. Click `Settings` in the top-right.
3. Choose Provider and Model.
4. Enter your API key for the selected provider.
5. Click `Save Key`.

Notes:

- API key is stored locally on your machine.
- You usually only need to set it once per provider.

## Daily Usage

1. Keep Sentinel open as a floating window.
2. Switch to the app/screen you want help with.
3. Type your question and press:
   - `Enter` to send
   - `Shift + Enter` for new line
4. Read the structured response in chat.

## Safety

- Sentinel is read-only.
- It does not click, type, or perform actions on your computer.
- It only captures screen context and provides suggestions.

## Troubleshooting

- `No API key configured for selected provider`
  - Open `Settings` and save a key for the current provider.

- Provider not available yet
  - Some providers/models are listed as coming soon in this build.
  - Use an available OpenAI model for now.

- Windows SmartScreen warning
  - Because Sentinel is a new app, Windows may show a warning.
  - Click `More info` → `Run anyway`.
