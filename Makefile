COMPOSE ?= docker compose

.PHONY: help dev watch up down restart logs ps test test-api test-web test-e2e test-all clean

help:
	@printf '%s\n' \
		'Targets:' \
		'  make dev          Build and run the full stack with Docker Compose Watch' \
		'  make watch        Alias for make dev' \
		'  make up           Build and run the full stack in the background' \
		'  make down         Stop containers' \
		'  make restart      Restart the full stack in the background' \
		'  make logs         Follow all container logs' \
		'  make ps           Show container status' \
		'  make test         Run fast Rust and Vitest gate tests inside containers' \
		'  make test-e2e     Run Cypress full-stack E2E tests' \
		'  make test-all     Run gate tests and Cypress E2E tests' \
		'  make clean        Stop containers and remove local volumes'

dev:
	$(COMPOSE) up --watch --build

watch: dev

up:
	$(COMPOSE) up -d --build

down:
	$(COMPOSE) down

restart: down up

logs:
	$(COMPOSE) logs -f

ps:
	$(COMPOSE) ps

test: up test-api test-web

test-api:
	$(COMPOSE) exec -T api cargo test --lib --bins

test-web:
	$(COMPOSE) exec -T web npm test

test-e2e:
	$(COMPOSE) up -d --build api-e2e web-e2e
	$(COMPOSE) run --rm cypress

test-all: test test-e2e

clean:
	$(COMPOSE) down --volumes --remove-orphans
