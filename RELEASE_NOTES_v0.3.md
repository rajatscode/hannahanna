# hannahanna v0.3.0

## New Features

### Command Aliases
```yaml
aliases:
  sw: switch
  ls: list
  lt: list --tree
```
- Cycle detection prevents infinite loops
- Chained aliases supported (s → sw → switch)

### Port Management
- `hn ports reassign <name>` - Reassign ports to resolve conflicts
- Shows before/after allocations

### Extended Hooks (7 total)
- `pre_create` - Before worktree creation
- `post_remove` - After worktree deletion
- `post_switch` - After switching
- `pre_integrate` / `post_integrate` - Around merges
- All support conditional execution via branch patterns

### State Management
- `hn state list` - View all state dirs with sizes
- `hn state clean` - Remove orphaned state
- `hn state size [name]` - Check disk usage

### Docker Enhancements
- `hn docker exec <name> <cmd>` - Execute in containers
- `hn docker restart <name>` - Restart containers
- `hn docker prune` - Clean orphaned resources

### Config Commands
- `hn config init` - Create config with template
- `hn config validate` - Check syntax
- `hn config show` - Display merged config
- `hn config edit` - Open in $EDITOR

### VCS Support
- Mercurial sparse checkout implemented
- Full Git, Mercurial, Jujutsu support

### Testing
- 247 integration tests (11 new for aliases)
- Zero warnings
- Full v0.3 coverage

## Breaking Changes
None. Fully backward compatible with v0.2.
