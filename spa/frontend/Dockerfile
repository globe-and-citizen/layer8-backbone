FROM node:20-alpine

# Set working directory
WORKDIR /spa/frontend

# Copy package files and install dependencies
COPY ./package*.json ./
COPY ./vite.config.ts ./
RUN npm install

# Copy source code
COPY . .

# Expose backend port (change if needed)
EXPOSE 5173

# Start the backend
CMD ["npm", "run", "dev"]