# Hannahanna v0.4 Performance Benchmarks

This document describes the benchmark suite for hannahanna and establishes performance targets for v0.4.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench list_worktrees

# Save baseline for comparison
cargo bench -- --save-baseline v0.4.0

# Compare against baseline
cargo bench -- --baseline v0.4.0
```

## Performance Targets

### 1. List Worktrees (100 worktrees)
**Target:** < 100ms

**What it measures:** Time to list all worktrees in a repository with 100 worktrees.

**Optimization:** Registry caching should reduce this time significantly by avoiding VCS queries on subsequent calls.

### 2. Create Worktree (no hooks)
**Target:** < 500ms

**What it measures:** Time to create a new worktree without executing any hooks.

**Baseline:** This establishes the raw VCS operation cost before adding hook execution overhead.

### 3. Fuzzy Search (1000 candidates)
**Target:** < 10ms

**What it measures:** Time to perform fuzzy matching on 1000 candidate worktree names.

**Current implementation:** The fuzzy matching algorithm should be fast enough for even large repositories.

### 4. Port Allocation (10 concurrent worktrees)
**Target:** < 2s

**What it measures:** Time to allocate unique ports for 10 worktrees with 3 services each (30 total port allocations).

**Note:** This includes file I/O for the port registry and conflict detection.

### 5. Config Load (hierarchy)
**Target:** < 50ms

**What it measures:** Time to load and merge configuration from the full hierarchy (system → user → repo → local).

**Optimization:** Config caching and lazy loading can improve this.

## Benchmark Results

### v0.4.0 Baseline (Date: TBD)

Run `cargo bench` to establish baseline metrics. Results will be stored in `target/criterion/`.

```bash
# Example output format:
# list_worktrees/10      time: [8.2ms 8.5ms 8.8ms]
# list_worktrees/50      time: [35ms 37ms 39ms]
# list_worktrees/100     time: [68ms 72ms 76ms] ✓ PASS (<100ms target)
# create_worktree_no_hooks  time: [425ms 445ms 465ms] ✓ PASS (<500ms target)
# fuzzy_search/1000      time: [2.1ms 2.3ms 2.5ms] ✓ PASS (<10ms target)
# port_allocation_concurrent_10  time: [1.2s 1.4s 1.6s] ✓ PASS (<2s target)
# config_load_hierarchy  time: [12ms 15ms 18ms] ✓ PASS (<50ms target)
```

## Optimization History

### Registry Caching (v0.4.0)
- **Before:** Every `hn list` command queries VCS backend
- **After:** Results cached with TTL/invalidation strategy
- **Expected improvement:** 50%+ reduction in list time for cache hits

### Parallel Operations (v0.3.0)
- **Feature:** `hn each --parallel` executes commands concurrently
- **Performance:** ~3x faster for 4+ worktrees (I/O bound operations)

## Continuous Benchmarking

To track performance regressions:

1. **Before making changes:**
   ```bash
   cargo bench -- --save-baseline before
   ```

2. **After making changes:**
   ```bash
   cargo bench -- --baseline before
   ```

3. **Review results:**
   - Criterion will show % change from baseline
   - Investigate any regressions > 5%

## Benchmark Implementation Notes

### Test Data Setup
- Benchmarks use `tempfile` for isolated test environments
- Git repositories are created fresh for each benchmark iteration
- Worktrees are created using actual `git worktree` commands for realism

### Limitations
- Benchmarks run in isolation and may not reflect real-world caching benefits
- File I/O performance varies by system (SSD vs HDD, filesystem type)
- Git operations may be slower on first run (OS-level caching effects)

### Future Benchmarks

Potential additions for v0.5:

- **Integration workflows** - Full `add → switch → integrate → remove` cycle
- **Hook execution** - Measure overhead of running hooks
- **Docker operations** - Container start/stop/restart timing
- **Memory profiling** - Track memory usage with large worktree counts
- **Sparse checkout** - Performance with different sparse patterns

## Contributing

When adding new features to hannahanna:

1. Consider adding a benchmark if the feature has performance implications
2. Run benchmarks before and after your changes
3. Document any performance characteristics in this file
4. Aim to meet or exceed the established targets
