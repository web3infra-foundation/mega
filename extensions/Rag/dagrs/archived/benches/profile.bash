#!/usr/bin/bash -e

bench_script="RUSTFLAGS=-g cargo bench \
    --features=bench-prost-codec \
    --bench compute_dag_bench"

# Profile the executable
bash -c "$bench_script -- --profile-time 10"

# Get the path to the executable from cargo
exe_path_string=$(bash -c "$bench_script --no-run 2>&1")
exe_path=$(echo $exe_path_string | sed -n 's/.*(\(.*\)).*/\1/p')

# Display the profile
/usr/local/go/pkg/tool/linux_amd64/pprof \
    -call_tree -lines \
    -output "./target/criterion/report/compute_dag_bench-profile.svg" -svg \
    "$exe_path" \
    "./target/criterion/compute dag/profile/profile.pb"
