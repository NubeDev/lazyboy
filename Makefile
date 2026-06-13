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
