# Max's Magic Mirror (MMM)

A customizable dashboard application built with SvelteKit, designed to display various widgets and information on a single screen.

## Features

- **Clock**: Time display
- **Weather**: Current weather and forecasts with alerts
- **Calendar**: Google Calendar integration with event display
- **System Statistics**: Monitor system performance (CPU, memory, etc.)
- **WiFi QR Code**: Generate QR codes for WiFi network sharing
- **Events**: Custom event displays with greetings (holidays, special occasions)
- **Greetings**: Time-based personalized greetings
- **Reminders**: Bin collection reminders and other notifications
- **RSS Feed**: Display RSS feed articles
- **AdGuard**: AdGuard Home statistics
- **Spotify**: Currently playing track display

## Technology Stack

- **Framework**: SvelteKit with TypeScript
- **Runtime**: Bun
- **Styling**: Tailwind CSS
- **Build Tool**: Vite

## Prerequisites

- Bun (latest version)

## Development

Install dependencies:

```sh
bun install
```

Start the development server:

```sh
bun run dev
```

The app will be available at `http://localhost:5173` (or the port shown in the terminal).

## Configuration

1. Copy the example config file:
   ```sh
   cp data/config.json.example data/config.json
   ```

2. Edit `data/config.json` to configure:
   - Component layout and positioning
   - API keys (weather, Google Calendar, Spotify, etc.)
   - Component-specific settings

## Building

Build for production:

```sh
bun run build
```

Preview the production build:

```sh
bun run preview
```

## Docker Deployment

Build the Docker image:

```sh
docker build -t mmm .
```

Run with Docker Compose:

```sh
docker-compose up -d
```

The app will be available at `http://localhost:4000`.

Make sure to mount the `data` directory to persist your configuration:
- `./data:/app/data` - Configuration files
- `./plugins:/app/plugins` - External plugins (optional)

## External Plugins

This application supports external plugins that can be added to extend functionality without modifying the core codebase.

### Plugin Structure

External plugins should be placed in the `plugins/` directory (mounted as a volume in Docker). Each plugin should be in its own subdirectory with the following structure:

```
plugins/
  my-plugin/
    manifest.json    # Required: Plugin manifest
    api.ts          # Optional: API handler (TypeScript)
    api.js          # Optional: API handler (JavaScript, alternative to api.ts)
```

### manifest.json

The manifest file defines the plugin's metadata and configuration schema:

```json
{
  "id": "my-plugin",
  "name": "My Custom Plugin",
  "version": "1.0.0",
  "description": "Description of what this plugin does",
  "config": {
    "title": "My Plugin Configuration",
    "description": "Configure your plugin settings",
    "fields": [
      {
        "key": "apiKey",
        "type": "text",
        "label": "API Key",
        "description": "Your API key for the service",
        "placeholder": "Enter your API key"
      },
      {
        "key": "refreshInterval",
        "type": "number",
        "label": "Refresh Interval (seconds)",
        "description": "How often to refresh the data",
        "default": 60
      }
    ]
  }
}
```

### API Handler (api.ts or api.js)

The API handler is a function that processes requests for your plugin. It should export a `GET` function (or default export) that matches this signature:

```typescript
import type { Request } from '@sveltejs/kit';

export async function GET(
  config: Record<string, unknown>,
  request?: Request
): Promise<Response> {
  // Your plugin logic here
  const apiKey = config.apiKey as string;
  const refreshInterval = config.refreshInterval as number;

  // Fetch data, process, etc.
  const data = await fetchSomeData(apiKey);

  return new Response(JSON.stringify(data), {
    headers: { 'Content-Type': 'application/json' }
  });
}
```

Or as a default export:

```typescript
export default async function handler(
  config: Record<string, unknown>,
  request?: Request
): Promise<Response> {
  // Your plugin logic
}
```

### Configuration Field Types

- `text`: Text input field
- `password`: Password input field (masked)
- `number`: Number input field
- `select`: Dropdown selection (requires `options` array)
- `array`: Array of items (requires `itemSchema` or `itemFields`)
- `color`: Color picker

#### Select Field Example

```json
{
  "key": "theme",
  "type": "select",
  "label": "Theme",
  "description": "Choose a theme",
  "options": [
    { "value": "light", "label": "Light" },
    { "value": "dark", "label": "Dark" }
  ]
}
```

### Important Notes

1. **Plugin IDs must be unique**: Plugin IDs cannot conflict with built-in component IDs. If a conflict is detected, the plugin will be skipped.

2. **Built-in components take precedence**: Built-in components are loaded first and cannot be overridden by plugins.

3. **API handlers only**: Currently, external plugins can only provide API handlers. UI components (Svelte files) are not yet supported for external plugins. If you need a custom UI component, it would need to be added to the built-in components.

4. **TypeScript/JavaScript support**: Plugins can be written in either TypeScript (`.ts`) or JavaScript (`.js`). Bun will handle TypeScript compilation automatically.

5. **Restart required**: After adding or modifying a plugin, you may need to restart the application for changes to take effect.

### Example Plugin

Here's a complete example of a simple plugin:

**plugins/example-plugin/manifest.json:**
```json
{
  "id": "example-plugin",
  "name": "Example Plugin",
  "version": "1.0.0",
  "description": "An example plugin that demonstrates the plugin system",
  "config": {
    "title": "Example Plugin Configuration",
    "description": "Configure the example plugin",
    "fields": [
      {
        "key": "message",
        "type": "text",
        "label": "Message",
        "description": "A message to display",
        "default": "Hello from plugin!"
      }
    ]
  }
}
```

**plugins/example-plugin/api.ts:**
```typescript
export async function GET(
  config: Record<string, unknown>
): Promise<Response> {
  const message = (config.message as string) || 'Hello from plugin!';

  return new Response(JSON.stringify({
    message,
    timestamp: new Date().toISOString()
  }), {
    headers: { 'Content-Type': 'application/json' }
  });
}
```

### Troubleshooting

- **Plugin not loading**: Check that the `manifest.json` file is valid JSON and contains all required fields (`id`, `name`, `config`).
- **API handler not working**: Ensure your handler exports a `GET` function or default export, and that it returns a `Response` object.
- **ID conflict**: Make sure your plugin ID doesn't match any built-in component ID. Check the console logs for warnings about conflicts.

## Project Structure

- `src/components/` - Widget components (clock, weather, calendar, etc.)
- `src/routes/api/` - Server-side API endpoints
- `src/lib/config/` - Configuration management
- `src/lib/core/` - Core utilities and types
- `data/config.json` - User configuration file

## Available Scripts

- `bun run dev` - Start development server
- `bun run build` - Build for production
- `bun run preview` - Preview production build
- `bun run check` - Type check with svelte-check
- `bun run lint` - Run linter
- `bun run format` - Format code with Prettier
