COMPOSE ?= docker compose

.PHONY: help dev watch up down restart logs ps test test-api test-web eval-upload clean

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
		'  make test         Run API and web gate tests inside containers' \
		'  make eval-upload  Run the MinIO upload smoke eval against a running stack' \
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
	$(COMPOSE) exec -T api cargo test

test-web:
	$(COMPOSE) exec -T web npm test

eval-upload: up
	bash docs/evals/minio-upload-smoke.sh

clean:
	$(COMPOSE) down --volumes --remove-orphans
