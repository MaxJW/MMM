# Use official Bun image
FROM oven/bun:1

# Set working directory inside container
WORKDIR /app

# Copy package files first for caching
COPY package.json bun.lock ./

# Install dependencies
RUN bun install

# Copy the rest of the project
COPY . .

# Run prepare scripts with full source available
RUN bun prepare

# Build the SvelteKit app
RUN bun run build

# Create plugins and data directories
RUN mkdir -p /app/plugins /app/data

# Expose the port the app will run on
EXPOSE 3000

# Run the app in production
CMD ["bun", "build/index.js"]
