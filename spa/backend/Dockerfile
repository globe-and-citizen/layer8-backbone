FROM node:20-alpine

# Set working directory
WORKDIR /spa/backend

# Copy package files and install dependencies
COPY ./package.json ./
RUN npm install

# Copy source code
COPY . .

# Expose backend port (change if needed)
EXPOSE 3000

# Start the backend
CMD ["node", "index.js"]