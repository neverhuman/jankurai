FROM node:20.11.1
WORKDIR /app
COPY src /app/src
USER node
HEALTHCHECK CMD node health.js
