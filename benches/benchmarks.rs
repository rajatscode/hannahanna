use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use hannahanna::config::Config;
use hannahanna::fuzzy;
use hannahanna::vcs::git::GitBackend;
use hannahanna::vcs::traits::VcsBackend;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Setup a test git repository with multiple worktrees
fn setup_test_repo(worktree_count: usize) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("test-repo");

    // Initialize git repo
    Command::new("git")
        .args(&["init", repo_path.to_str().unwrap()])
        .output()
        .unwrap();

    // Configure git
    Command::new("git")
        .args(&["-C", repo_path.to_str().unwrap(), "config", "user.email", "test@example.com"])
        .output()
        .unwrap();

    Command::new("git")
        .args(&["-C", repo_path.to_str().unwrap(), "config", "user.name", "Test User"])
        .output()
        .unwrap();

    // Create initial commit
    fs::write(repo_path.join("README.md"), "# Test Repo").unwrap();
    Command::new("git")
        .args(&["-C", repo_path.to_str().unwrap(), "add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .args(&["-C", repo_path.to_str().unwrap(), "commit", "-m", "Initial commit"])
        .output()
        .unwrap();

    // Create worktrees
    for i in 0..worktree_count {
        let worktree_name = format!("feature-{}", i);
        let worktree_path = temp_dir.path().join(&worktree_name);
        Command::new("git")
            .args(&[
                "-C",
                repo_path.to_str().unwrap(),
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                "-b",
                &worktree_name,
            ])
            .output()
            .unwrap();
    }

    (temp_dir, repo_path)
}

/// Benchmark: List worktrees with varying counts
fn bench_list_worktrees(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_worktrees");

    for count in [10, 50, 100].iter() {
        let (_temp_dir, repo_path) = setup_test_repo(*count);

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                std::env::set_current_dir(&repo_path).unwrap();
                let backend = GitBackend::open_from_current_dir().unwrap();
                let worktrees = backend.list_workspaces().unwrap();
                black_box(worktrees)
            });
        });
    }

    group.finish();
}

/// Benchmark: Create single worktree (no hooks)
fn bench_create_worktree_no_hooks(c: &mut Criterion) {
    c.bench_function("create_worktree_no_hooks", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let repo_path = temp_dir.path().join("test-repo");

                // Initialize git repo
                Command::new("git")
                    .args(&["init", repo_path.to_str().unwrap()])
                    .output()
                    .unwrap();

                // Configure git
                Command::new("git")
                    .args(&["-C", repo_path.to_str().unwrap(), "config", "user.email", "test@example.com"])
                    .output()
                    .unwrap();
                Command::new("git")
                    .args(&["-C", repo_path.to_str().unwrap(), "config", "user.name", "Test User"])
                    .output()
                    .unwrap();

                // Create initial commit
                fs::write(repo_path.join("README.md"), "# Test Repo").unwrap();
                Command::new("git")
                    .args(&["-C", repo_path.to_str().unwrap(), "add", "."])
                    .output()
                    .unwrap();
                Command::new("git")
                    .args(&["-C", repo_path.to_str().unwrap(), "commit", "-m", "Initial commit"])
                    .output()
                    .unwrap();

                (temp_dir, repo_path)
            },
            |(temp_dir, repo_path)| {
                let worktree_path = temp_dir.path().join("new-feature");
                Command::new("git")
                    .args(&[
                        "-C",
                        repo_path.to_str().unwrap(),
                        "worktree",
                        "add",
                        worktree_path.to_str().unwrap(),
                        "-b",
                        "new-feature",
                    ])
                    .output()
                    .unwrap();
                black_box(worktree_path)
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark: Fuzzy search with varying candidate counts
fn bench_fuzzy_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuzzy_search");

    for count in [100, 500, 1000].iter() {
        let candidates: Vec<String> = (0..*count)
            .map(|i| format!("feature-branch-{}-with-long-name", i))
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let result = fuzzy::find_best_match(
                    black_box("feat-500"),
                    black_box(&candidates),
                );
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark: Config loading from hierarchy
fn bench_config_load_hierarchy(c: &mut Criterion) {
    c.bench_function("config_load_hierarchy", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let repo_path = temp_dir.path().join("test-repo");
                fs::create_dir_all(&repo_path).unwrap();

                // Create .git directory to mark as repo root
                fs::create_dir_all(repo_path.join(".git")).unwrap();

                // Create config file
                let config_content = r#"
docker:
  enabled: true
  compose_file: docker-compose.yml
hooks:
  post_create: |
    echo "Setup complete"
"#;
                fs::write(repo_path.join(".hannahanna.yml"), config_content).unwrap();

                (temp_dir, repo_path)
            },
            |(_temp_dir, repo_path)| {
                let config = Config::load(&repo_path).unwrap();
                black_box(config)
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark: Port allocation for concurrent worktrees
fn bench_port_allocation_concurrent(c: &mut Criterion) {
    use hannahanna::docker::ports::PortAllocator;

    c.bench_function("port_allocation_concurrent_10", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().unwrap();
                let state_dir = temp_dir.path().join(".hn-state");
                fs::create_dir_all(&state_dir).unwrap();
                (temp_dir, state_dir)
            },
            |(_temp_dir, state_dir)| {
                let mut allocator = PortAllocator::new(&state_dir).unwrap();

                // Allocate ports for 10 worktrees
                for i in 0..10 {
                    let worktree_name = format!("feature-{}", i);
                    let services = &["app", "db", "redis"];
                    allocator.allocate(&worktree_name, services).unwrap();
                }

                black_box(allocator)
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_list_worktrees,
    bench_create_worktree_no_hooks,
    bench_fuzzy_search,
    bench_config_load_hierarchy,
    bench_port_allocation_concurrent,
);
criterion_main!(benches);
