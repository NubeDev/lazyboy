# Lazyboy tooling.
#
# Goose is rented, unforked, and pinned (SCOPE.md "Runtime decision").
# We vendor the release binary into bin/ rather than relying on a
# system-wide install, so every node runs the exact version we tested
# against. Bump GOOSE_VERSION deliberately, never float it.

GOOSE_VERSION := 1.37.0
GOOSE_BIN     := bin/goose

# block/goose ships one binary; the headless server the bridge drives
# is `goose serve` (ACP over HTTP + WebSocket).
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

ifeq ($(UNAME_S),Linux)
  GOOSE_TARGET := $(UNAME_M)-unknown-linux-gnu
endif
ifeq ($(UNAME_S),Darwin)
  ifeq ($(UNAME_M),arm64)
    GOOSE_TARGET := aarch64-apple-darwin
  else
    GOOSE_TARGET := x86_64-apple-darwin
  endif
endif

GOOSE_ARCHIVE := goose-$(GOOSE_TARGET).tar.bz2
GOOSE_URL     := https://github.com/block/goose/releases/download/v$(GOOSE_VERSION)/$(GOOSE_ARCHIVE)

.PHONY: install-goose goose-version clean-goose

install-goose: $(GOOSE_BIN)
	@$(GOOSE_BIN) --version

$(GOOSE_BIN):
	@test -n "$(GOOSE_TARGET)" || { echo "unsupported platform: $(UNAME_S) $(UNAME_M)"; exit 1; }
	@mkdir -p bin
	@echo "fetching goose v$(GOOSE_VERSION) ($(GOOSE_TARGET))"
	@tmp=$$(mktemp -d); \
	  curl -fsSL -o $$tmp/$(GOOSE_ARCHIVE) "$(GOOSE_URL)"; \
	  tar xjf $$tmp/$(GOOSE_ARCHIVE) -C $$tmp; \
	  install -m 0755 $$tmp/goose $(GOOSE_BIN); \
	  rm -rf $$tmp
	@echo "installed -> $(GOOSE_BIN)"

goose-version:
	@$(GOOSE_BIN) --version 2>/dev/null || echo "goose not installed (run: make install-goose)"

clean-goose:
	@rm -f $(GOOSE_BIN)

# Backend (the lazyboy-server HTTP+SSE shell over the core). The env
# defaults here mirror crates/lazyboy-server/src/main.rs so `make
# backend-start` and a bare `lazyboy-server` behave identically; override
# any of them on the command line, e.g. `make backend-start LAZYBOY_ADDR=0.0.0.0:9000`.
# (The goose serve url is LAZYBOY_GOOSE_URL here, not GOOSE_URL, because
# GOOSE_URL is already the install-goose download url above; it is
# exported to the server as GOOSE_URL at launch.)
LAZYBOY_DB        ?= lazyboy.db
LAZYBOY_ADDR      ?= 127.0.0.1:7878
LAZYBOY_GOOSE_URL ?= http://127.0.0.1:3284
LAZYBOY_TOKEN     ?=

# The server reads the db that `lazyboy init` bootstrapped; the sidecar
# config (lazyboy.json) marks a db as initialized. We start the server
# detached and record its pid so `backend-stop` can find it without a
# port scan or pkill guesswork.
SERVER_PID := .lazyboy-server.pid
SERVER_LOG := .lazyboy-server.log

.PHONY: backend-build backend-init backend-start backend-stop backend-status backend-logs

backend-build:
	cargo build --release -p lazyboy-cli -p lazyboy-server

# Bootstrap the workspace/space once. Idempotent: the CLI refuses to
# clobber an existing db, so re-running is a no-op rather than an error.
backend-init: backend-build
	@test -f $(patsubst %.db,%.json,$(LAZYBOY_DB)) \
	  && echo "already initialized ($(LAZYBOY_DB))" \
	  || LAZYBOY_DB=$(LAZYBOY_DB) ./target/release/lazyboy init

backend-start: backend-init
	@if [ -f $(SERVER_PID) ] && kill -0 $$(cat $(SERVER_PID)) 2>/dev/null; then \
	  echo "already running (pid $$(cat $(SERVER_PID))); run 'make backend-stop' first"; exit 1; \
	fi
	@LAZYBOY_DB=$(LAZYBOY_DB) LAZYBOY_ADDR=$(LAZYBOY_ADDR) GOOSE_URL=$(LAZYBOY_GOOSE_URL) \
	  LAZYBOY_TOKEN="$(LAZYBOY_TOKEN)" \
	  ./target/release/lazyboy-server > $(SERVER_LOG) 2>&1 & echo $$! > $(SERVER_PID)
	@echo "lazyboy-server started (pid $$(cat $(SERVER_PID))) on http://$(LAZYBOY_ADDR), logging to $(SERVER_LOG)"

backend-stop:
	@if [ -f $(SERVER_PID) ]; then \
	  kill $$(cat $(SERVER_PID)) 2>/dev/null && echo "stopped pid $$(cat $(SERVER_PID))" \
	    || echo "process $$(cat $(SERVER_PID)) not running"; \
	  rm -f $(SERVER_PID); \
	else \
	  echo "no pid file; nothing to stop"; \
	fi

backend-status:
	@if [ -f $(SERVER_PID) ] && kill -0 $$(cat $(SERVER_PID)) 2>/dev/null; then \
	  echo "running (pid $$(cat $(SERVER_PID))) on http://$(LAZYBOY_ADDR)"; \
	else \
	  echo "not running"; \
	fi

backend-logs:
	@tail -f $(SERVER_LOG)
