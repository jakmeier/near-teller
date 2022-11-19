.PHONY: test
test: rebuild
	cargo test
	cargo test --test integration-tests

.PHONY: rebuild
rebuild: res/near_teller.wasm

res/near_teller.big.wasm: src/*
	cargo build -r -p near-teller --target wasm32-unknown-unknown
	cp $${CARGO_TARGET_DIR}/wasm32-unknown-unknown/release/near_teller.wasm $@

%.wasm: %.big.wasm
	wasm-opt -Os $< -o $@
	wasm-strip $@
