export RUST_BACKTRACE=full
export RUST_LOG=debug
export DEBUG=true
export REDIS_PASSWORD=7tgbBSO2Yu
#export REDIS_SENTINEL_MASTER=mymaster
export REDIS_ADDRESS=localhost:6379
export REDIS_SENTINEL_ADDRESSES=localhost:26379
export PRIVATE_PORT=3003
export PUBLIC_PORT=3004
export NAMESPACE=rhiaqey

define CHANNELS
[
	{
		"Name": "sdf",
		"Size": 10
	},
	{
		"Name": "cokoland",
		"Size": 15
	}
]
endef

export CHANNELS

.PHONY: ws1
ws1:
	ID=ws1 \
	NAME=ws-1 \
		cargo +nightly run --bin websocket --features=websocket

.PHONY: build
build:
	cargo +nightly build

.PHONY: prod
prod:
	cargo +nightly build --release

.PHONY: redis
redis:
	docker run -it --rm --name redis -p 6379:6379 \
		-e ALLOW_EMPTY_PASSWORD=yes \
		bitnami/redis:7.0.9
