# Use official Bun image
FROM oven/bun:latest

# Set working directory inside container
WORKDIR /app

# Copy package files first for caching
COPY package.json bun.lock ./

# Install dependencies
RUN bun install

# Copy the rest of the project
COPY . .

# Build the SvelteKit app
RUN bun run build

# Expose the port the app will run on
EXPOSE 4000

# Run the app in production
CMD ["bun", "build/index.js"]
