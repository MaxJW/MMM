# Magic Mirror Dashboard

A modern dashboard application built with SvelteKit, designed to display various widgets like weather, RSS feeds, system stats, and more.

## Features

- **Weather Widget**: Current weather and forecasts
- **RSS Feed Reader**: Display RSS feeds
- **System Statistics**: Monitor system performance
- **Clock & Greetings**: Time and personalized greetings
- **Events & Reminders**: Calendar and reminder management
- **Bin Collections**: Waste collection schedules

## Technology Stack

- **Frontend**: SvelteKit with TypeScript
- **Styling**: Tailwind CSS
- **Runtime**: Node.js
- **Build Tool**: Vite

## Development

Install dependencies:

```sh
npm install
```

Start the development server:

```sh
npm run dev

# or start the server and open the app in a new browser tab
npm run dev -- --open
```

## Building

To create a production version of your app:

```sh
npm run build
```

You can preview the production build with `npm run preview`.

## Project Structure

- `src/lib/components/` - Svelte components for each widget
- `src/lib/services/` - API services for external data
- `src/routes/api/` - Server-side API endpoints
- `src/lib/config/` - Configuration files
- `src/lib/types/` - TypeScript type definitions

## Deployment

This project uses `@sveltejs/adapter-node` for Node.js deployment. The built application can be deployed to any Node.js hosting platform.
