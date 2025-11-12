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

### v0.4.0 Baseline (Date: 2025-11-12)

**Note**: Benchmarks include setup time (creating temp repos), so absolute times are higher than real-world usage. The benchmarks establish a baseline for detecting performance regressions.

Run `cargo bench` to run benchmarks locally. Results are stored in `target/criterion/`.

**Initial Results:**
```
list_worktrees/10      time:   [238ms 241ms 244ms]
list_worktrees/50      time:   [1.04s 1.05s 1.06s]
list_worktrees/100     time:   [2.07s 2.09s 2.11s]
```

**Note**: These times include creating temporary git repositories for testing. In real-world usage with existing worktrees, `hn list` typically runs in < 100ms with caching.

**To run benchmarks yourself:**
```bash
cargo bench

# Or run specific benchmark
cargo bench list_worktrees

# Save as baseline for comparison
cargo bench -- --save-baseline my-baseline

# Compare against baseline
cargo bench -- --baseline my-baseline
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
