run:
	cargo build --release --no-default-features
	cd ../../ && cargo run --release -- --release evals/astarpa/experiments/*.yaml --cache evals/astarpa/results/cache.json

run_release: cpu-freq prep run

export_plots:
	cp -r plots ../../../../astarpa-paper/plots

# NOTE: BIOS settings used:
# - no hyperthreading
# - no turbo / frequency boost
# - disabled power saving CPU for CPU
# - `balanced` performance, even with A/C power.
#   - `performance` leads to throttling
# - laptop plugged in

cpu-freq:
	@echo -e "Sudo is needed to call 'cpupower frequency-set' to pin the cpu frequency.\n"
	sudo cpupower frequency-set -d 2.6GHz -u 2.6GHz -g powersave

# Kill some background programs.
prep:
	systemctl --user stop emacs || true
	/usr/bin/kill signal-desktop || true
	/usr/bin/kill chromium || true
	/usr/bin/kill telegram-desktop || true
	/usr/bin/kill slack || true

zip_results:
	zip results.zip results/*.json
unzip_results:
	unzip results.zip
