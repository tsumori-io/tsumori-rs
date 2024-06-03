# Also from Reth: https://github.com/paradigmxyz/reth/blob/7da36e042125397af89b9b477f9292dc676e69f8/Makefile

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: cargo-setup
cargo-setup: ## Install Cargo dependencies.
	rustup toolchain install nightly
	rustup component add rustfmt --toolchain nightly
