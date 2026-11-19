# Publishing ClipStack

One-time setup, then every release is `git tag vX.Y.Z && git push --tags`.

## 1. GitHub repository

On github.com → **New repository** → name `clipstack` (public), no README/licence (the repo
already has them). Then, from this folder:

```bash
git remote add origin git@github.com:<your-user>/clipstack.git
git push -u origin main
```

(Use the HTTPS URL instead if you don't have SSH keys on GitHub.)

The `.github/workflows/release.yml` workflow is picked up automatically.

## 2. npm account and scope

The binaries are published as `@clipstack/<os>-<arch>` and the installer as `clipstack`.

1. Create / sign in to your npm account at npmjs.com.
2. Create an **organisation** named `clipstack` (npmjs.com → avatar → *Add Organization* →
   free/public). That reserves the `@clipstack` scope.
   *If the name is taken, pick another scope (e.g. `@sharbuz`) and tell Claude — the five
   `optionalDependencies` in `cross/npm/clipstack/package.json` and the scope in
   `cross/npm/scripts/make-platform-package.js` need the same change.*
3. Create an **automation token**: avatar → *Access Tokens* → *Generate New Token* →
   *Granular* → packages & scopes: read+write on `clipstack` and `@clipstack/*`.
   Copy it; it is shown once.

## 3. GitHub secret

Repository → **Settings → Secrets and variables → Actions → New repository secret**:
`NPM_TOKEN` = the token from step 2.

## 4. First release

```bash
git tag v0.1.0
git push --tags
```

Watch **Actions** on GitHub. The workflow:

- builds macOS (arm64, x64), Windows (x64), Linux (x64, arm64) binaries — about 10 minutes,
- creates a GitHub Release `v0.1.0` with a `.tar.gz` per platform,
- publishes the five `@clipstack/*` packages, then `clipstack` itself.

Then on any machine:

```bash
npx clipstack install
```

## 5. Next releases

Bump nothing by hand: the workflow stamps the version from the tag into every package.
Just commit, tag `v0.1.1` (or whatever), push the tag.

## Before the first tag — manual tests still owed

| Platform | How to test now |
|---|---|
| macOS | the Swift app is installed; the Rust binary is `cross/target/release/clipstack` |
| Linux | copy `cross/target-linux/release/clipstack` (arm64) to a Linux box, or wait for the x64 CI artifact; run it, check the tray, `Ctrl+Alt+V` |
| Windows | no binary yet — the first CI run produces it (download from the Actions run's artifacts, or from the release) |

Expect a round of fixes after the first real Windows/Linux run; nothing there has been clicked yet.

## Optional: signed macOS binary from CI

Unsigned builds make macOS re-ask for Accessibility after each update. To sign in CI:
export the *Developer ID Application* certificate as a `.p12`, add it (base64) and its
password as secrets `MAC_CERT_P12` / `MAC_CERT_PASSWORD`, and add a `codesign` step after
the macOS build. Ask Claude to wire it when you want it.
