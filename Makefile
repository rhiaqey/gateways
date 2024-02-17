export RUST_BACKTRACE=full
export RUST_LOG=debug
export DEBUG=true
export REDIS_PASSWORD=welcome
#export REDIS_SENTINEL_MASTER=mymaster
export REDIS_ADDRESS=localhost:6379
export REDIS_SENTINEL_ADDRESSES=localhost:26379
export PRIVATE_PORT=3010
export PUBLIC_PORT=3011
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

define PUBLIC_KEY
-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEAwWOo7UYK8upVY3qf1zvpwdyVL+4KWwKx4lKQXd5ljiEjNBdhQlRP
869LFR+k4CMIYqKYSGzbYpvfXOwXNHEjfwXiEnm8gro8cGTRdb7n9jKpN7UXMIez
DRflWd8K8Cma4DQPethmNiCtpMoHlINYgNMFTtbK9QOaFKO1JZVUyrHN+qsmtkPO
dMJ68zHiQMtWs00eABdPtS3cSmvfkk7Dz30pcNXdHuYtEQx3KAfqRIJ49F1vnu17
D6Sw5fRD+IxhICkuKjATBqAkpYk5sP4Xcd2QYqU8qFTmu3WrwgHEHAZ85L3kNxKb
Hh3mF9PnLxlo4WQANH65Ej51HVGOKw4VHQIDAQAB
-----END RSA PUBLIC KEY-----
endef

export PUBLIC_KEY

define PRIVATE_KEY
-----BEGIN RSA PRIVATE KEY-----
MIIEpQIBAAKCAQEAwWOo7UYK8upVY3qf1zvpwdyVL+4KWwKx4lKQXd5ljiEjNBdh
QlRP869LFR+k4CMIYqKYSGzbYpvfXOwXNHEjfwXiEnm8gro8cGTRdb7n9jKpN7UX
MIezDRflWd8K8Cma4DQPethmNiCtpMoHlINYgNMFTtbK9QOaFKO1JZVUyrHN+qsm
tkPOdMJ68zHiQMtWs00eABdPtS3cSmvfkk7Dz30pcNXdHuYtEQx3KAfqRIJ49F1v
nu17D6Sw5fRD+IxhICkuKjATBqAkpYk5sP4Xcd2QYqU8qFTmu3WrwgHEHAZ85L3k
NxKbHh3mF9PnLxlo4WQANH65Ej51HVGOKw4VHQIDAQABAoIBAFUt0kAAM95evJF+
d1zT9NgAkm10CXegrj0jZJcT1+NMUTcmfR48CKMquIVrVLGsfIsFVtG/sLm0MiO5
kVb15k6Shsrgd9mUsf5HScL0/TKBiesRhk9H1eOUfN6i0SyLBr5t78uJ+SsqJZGJ
suEFITxMte/Nx8M3fOxOVwFgzuIfvrn/b5ylsWHOVaKv3nH5wIXRenBUHHSoW/93
FUDyr4ylk3K0ipYkCKCK2ZU/C/GfJSWPiWeJCA+8u9avZDc4Jo5VgNg1bAGeISjI
DW5K6MfKZQRUMf5Yxp7o8Vn0KCVLCZ135OUATt87LrpAcuVhohk5f95zwwg+RDHV
a2BJPHkCgYEA3hz4DFRWT9SL6ISqwmgKLG4TPt7BCIGudS5rUI4usFpPxclJ92d/
anbrbaysyQooFHZbSjLPrx+mAyoyl8X/o4Vg7dYFRqqYrR0qIDNlsSFCLmFoPJsf
iq9aMdvbCZ4fgiU+hhFqf9ggwi4cfBLJ0dLRkNSvvT1WkjAqyh2K5sMCgYEA3uTU
GPsF5pHiU1l15EBWb7cQHbnst67nx6wAF1NgFbvnKsloILjHhvJt2v3y6eHVzPmz
eTP5eX0NGkoxSM+XUGAEwoojj0N60oQfHq89D8l1FMnnycVrYNbPpcpcj06InCI6
DD8v/eegdtJB72nRRVWYVPJEbQMijpXe1L78Fp8CgYEAux+yxkhjMvxBJpJmfkRC
le3inuvxuqXugWCrHoG6ye48GMidXSa/zMUFUS/RncnFvH9+J4OpPsOuDCqH1yAD
YBMldxoA9ekRmX7hl6FVgiYf6I090RlrOF7E4Q51eaPSrcWM6ExR6gT+jDlm3AzE
JNa0oYzdxdOgVKbp1b+P3xsCgYEAxbrFlOuKzoH3/uzLspKIm04Qk+5N2pzkrLMe
2ZJzGJS6e7B0GSMSjdoeLjk99tEKs22IEytSUr3mk73hfq/5kam8Tz/wT7UTDhF6
8eOPDaQvoyOB5fKmUR/+0Rp1hgOrGKccS6T6VAnYxc+8AkEjDpjiK+lHXlV7oHW/
WYebZg0CgYEA05xqLd7BviuWt0alFWxn3d73fHfDxMljk81w3/7XraRotiLsvoFG
65/r/v/BefLY8aJmIRuZC26HYyDgNXoe+h9Izp4mg+5PXpTWJx5Lx43/yErIg+HG
Xzq0hM31N+8073JWS+QRP1YJJWHqDhJbTJOzK2VepVubjEmPTY1DeTw=
-----END RSA PRIVATE KEY-----
endef

export PRIVATE_KEY

.PHONY: ws1
ws1:
	ID=ws1 \
	NAME=ws-1 \
		cargo run --bin websocket --features=websocket

.PHONY: build
build:
	cargo build -j 64

.PHONY: dev
dev:
	cargo build --features=all -j 64

.PHONY: prod
prod:
	cargo build --release --features=all -j 64

.PHONY: redis
redis:
	docker run -it --rm --name redis -p 6379:6379 \
		-e ALLOW_EMPTY_PASSWORD=yes \
		bitnami/redis:7.2.4

.PHONY: sentinel
sentinel:
	docker run -it --rm --name redis-sentinel -p 26379:26379 \
		-e ALLOW_EMPTY_PASSWORD=yes \
		-e REDIS_MASTER_HOST=localhost \
		bitnami/redis-sentinel:7.2.4

.PHONY: sentinel2
sentinel2:
	docker run -it --rm --name redis-sentinel-2 -p 26380:26379 \
		-e ALLOW_EMPTY_PASSWORD=yes \
		-e REDIS_MASTER_HOST=localhost \
		bitnami/redis-sentinel:7.2.4
