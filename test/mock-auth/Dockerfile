FROM node:20-alpine

WORKDIR /app

COPY package.json ./
RUN apk add --no-cache wget
RUN npm install --omit=dev

COPY index.js ./

EXPOSE 5001

CMD ["node", "index.js"]
