SHELL := /bin/bash
RUST_LOG ?= prusti=info
RUST_TEST_THREADS ?= 1
JAVA_HOME ?= /usr/lib/jvm/default-java
RUN_FILE ?= prusti/tests/typecheck/pass/lint.rs
RUN_FILE_FOLDER=$(shell dirname ${RUN_FILE})
JAVA_LIBJVM_DIR=$(shell dirname "$(shell find "$(shell readlink -f ${JAVA_HOME})" -name "libjvm.so")")
RUSTUP_TOOLCHAIN=$(shell cat rust-toolchain)
RUST_VERSION = ${RUSTUP_TOOLCHAIN}-x86_64-unknown-linux-gnu
COMPILER_PATH = $$HOME/.rustup/toolchains/${RUST_VERSION}
LIB_PATH = ${COMPILER_PATH}/lib:${JAVA_LIBJVM_DIR}:./target/debug:./target/debug/deps
RELEASE_LIB_PATH = ${COMPILER_PATH}/lib:${JAVA_LIBJVM_DIR}:./target/release:./target/release/deps
PRUSTI_DRIVER=./target/debug/prusti-driver
PRUSTI_DRIVER_RELEASE=./target/release/prusti-driver

SET_ENV_VARS = LD_LIBRARY_PATH=$(LIB_PATH) JAVA_HOME=$(JAVA_HOME) RUST_TEST_THREADS=$(RUST_TEST_THREADS)

SET_RELEASE_ENV_VARS = LD_LIBRARY_PATH=$(RELEASE_LIB_PATH) JAVA_HOME=$(JAVA_HOME) RUST_TEST_THREADS=$(RUST_TEST_THREADS)

default: build

fmt:
	rustup run nightly cargo fmt --all || true

fix:
	$(SET_ENV_VARS) cargo fix

check:
	$(SET_ENV_VARS) cargo check --all

build:
	$(SET_ENV_VARS) cargo build --all

release:
	$(SET_ENV_VARS) cargo build --release --all

test-deep:
	$(SET_ENV_VARS) \
	PRUSTI_CHECK_FOLDUNFOLD_STATE=1 \
	cargo test --all

test:
	$(SET_ENV_VARS) \
	cargo test --all

test-examples:
	$(SET_ENV_VARS) \
	cargo test -p prusti

build-profile:
	$(SET_ENV_VARS) \
	CARGO_INCREMENTAL=0 \
	RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Zno-landing-pads" \
	cargo build --all

long-test: build
	find prusti/tests/verify/long-pass/ -name '*.rs' | while read run_file; do \
		echo "Testing '$$run_file'..."; \
		START=$$(date +%s); \
		$(SET_ENV_VARS) RUST_BACKTRACE=1\
		$(PRUSTI_DRIVER) \
			-L ${COMPILER_PATH}/lib/rustlib/x86_64-unknown-linux-gnu/lib/ \
			--extern prusti_contracts=$(wildcard ./target/debug/deps/libprusti_contracts-*.rlib) \
			"$$run_file" \
			|| exit 1; \
		END=$$(date +%s); \
		DIFF=$$(( $$END - $$START )); \
		echo "$$run_file $$DIFF" >> timings; \
	done

long-test-overflow: build
	find prusti/tests/verify/long-pass-overflow/ -name '*.rs' | while read run_file; do \
		echo "Testing '$$run_file'..."; \
		START=$$(date +%s); \
		$(SET_ENV_VARS) RUST_BACKTRACE=1 \
		PRUSTI_CHECK_BINARY_OPERATIONS=1 \
		$(PRUSTI_DRIVER) \
			-L ${COMPILER_PATH}/lib/rustlib/x86_64-unknown-linux-gnu/lib/ \
			--extern prusti_contracts=$(wildcard ./target/debug/deps/libprusti_contracts-*.rlib) \
			"$$run_file" \
			|| exit 1; \
		END=$$(date +%s); \
		DIFF=$$(( $$END - $$START )); \
		echo "$$run_file $$DIFF" >> timings; \
	done

bench:
	$(SET_ENV_VARS) cargo bench --all

run: build
	$(SET_ENV_VARS) RUST_LOG=$(RUST_LOG) \
	$(PRUSTI_DRIVER) \
		-L ${COMPILER_PATH}/lib/rustlib/x86_64-unknown-linux-gnu/lib/ \
		--extern prusti_contracts=$(wildcard ./target/debug/deps/libprusti_contracts-*.rlib) \
		$(RUN_FILE)

run-flamegraph: build
	$(SET_ENV_VARS) RUST_LOG=$(RUST_LOG) \
    perf record -F 99 --call-graph=dwarf,32000 \
	$(PRUSTI_DRIVER) \
		-L ${COMPILER_PATH}/lib/rustlib/x86_64-unknown-linux-gnu/lib/ \
		--extern prusti_contracts=$(wildcard ./target/debug/deps/libprusti_contracts-*.rlib) \
		$(RUN_FILE)
	@echo "Now run 'flamegraph-rust-perf > flame.svg'"

run-release: release
	$(SET_RELEASE_ENV_VARS) RUST_LOG=$(RUST_LOG) \
	$(PRUSTI_DRIVER_RELEASE) \
		-L ${COMPILER_PATH}/lib/rustlib/x86_64-unknown-linux-gnu/lib/ \
		--extern prusti_contracts=$(wildcard ./target/release/deps/libprusti_contracts-*.rlib) \
		$(RUN_FILE)

run-release-profile: release
	$(SET_RELEASE_ENV_VARS) RUST_LOG=$(RUST_LOG) \
    valgrind --tool=callgrind --vex-iropt-register-updates=allregs-at-mem-access \
	${PRUSTI_DRIVER_RELEASE} \
		-L ${COMPILER_PATH}/lib/rustlib/x86_64-unknown-linux-gnu/lib/ \
		--extern prusti_contracts=$(wildcard ./target/release/deps/libprusti_contracts-*.rlib) \
		${RUN_FILE}
	@echo "Now run 'kcachegrind callgrind.out.*'"

run-release-flamegraph: release
	$(SET_RELEASE_ENV_VARS) RUST_LOG=$(RUST_LOG) \
    perf record -F 99 --call-graph=dwarf,32000 \
	${PRUSTI_DRIVER_RELEASE} \
		-L ${COMPILER_PATH}/lib/rustlib/x86_64-unknown-linux-gnu/lib/ \
		--extern prusti_contracts=$(wildcard ./target/release/deps/libprusti_contracts-*.rlib) \
		${RUN_FILE}
	@echo "Now run 'flamegraph-rust-perf > flame.svg'"

run-release-prusti-rustc-flamegraph: release
	perf record -F 99 --call-graph=dwarf,32000 \
	    ./target/release/prusti-rustc ${RUN_FILE}
	@echo "Now run 'flamegraph-rust-perf > flame.svg'"

run-release-prusti-rustc-timechart: release
	perf timechart record \
	    ./target/release/prusti-rustc ${RUN_FILE}
	@echo "Now run 'perf timechart'"

run-release-prusti-rustc-timechart-io: release
	perf timechart record -I \
	    ./target/release/prusti-rustc ${RUN_FILE}
	@echo "Now run 'perf timechart'"

update:
	cargo update

docs: update
	$(SET_ENV_VARS) cargo doc --all

clippy: clean
	$(SET_ENV_VARS) cargo clippy --all

publish-docker-images:
	docker push fpoli/prusti-base

build-docker-images:
	docker build -t fpoli/prusti-base --build-arg RUST_TOOLCHAIN="${RUSTUP_TOOLCHAIN}" -f docker/base.Dockerfile docker/
	docker build -t rust-nightly -f docker/playground.Dockerfile .

clean:
	cargo clean
	find . -name '*.bk' -delete
	rm -rf log
	rm -rf nll-facts
	rm -rf prusti/log
	rm -rf prusti/nll-facts

todo:
	git grep -i "todo\|fixme\|xxx\|hack"

demo:
	vagrant up
