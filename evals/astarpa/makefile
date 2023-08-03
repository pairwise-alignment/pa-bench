# NOTE: Stats collection for timing was added after main evals were done, and
# use a separate cache because timings work out slightly differently.
run:
	cargo build --release --no-default-features
    # Note: this one runs in parallel, unlike the others, to speed things up a bit.
	cd ../../ && cargo run --release -- bench -j=5 --nice=-20 evals/astarpa/experiments/{params,params_real}.yaml --cache evals/astarpa/results/cache_params.json
	cd ../../ && cargo run --release -- bench --release evals/astarpa/experiments/{scaling_n,scaling_e,real,memory,timing,timing_real,tools}.yaml --cache evals/astarpa/results/cache.json

run_release: cpu-freq prep run

build_plots:
	rm -f plots/*
	jupyter execute evals.ipynb

export_plots:
	rm -f ../../../../astarpa-paper/plots/*.pdf
	cp -r plots/*.pdf ../../../../astarpa-paper/plots/

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
