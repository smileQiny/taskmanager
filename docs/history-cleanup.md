# Git history cleanup

This repository's history contains generated files that were committed early and later gitignored. A normal clone of `main` is about **408 MiB** even though the current tree is small. The cleaned history is about **845 KiB**.

Merging the documentation pull request does not shrink the repository. The cleaned commits are on `cursor/filter-repo-cleaned-f1f7`. The owner has to publish that history onto `main` with a force-push. This change set does not do that, and it does not delete any remote branch.

Product sync code is unchanged. The rewritten `main` tree matches the pre-cleanup `main` tree byte for byte.

## What was measured

Measurements were taken on 2026-10-19 with Git 2.43.0 and git-filter-repo 2.47.0.

| Clone | Packed size | On disk | Packed objects | Commits on `main` |
| --- | ---: | ---: | ---: | ---: |
| Before: `git clone --no-local --bare --single-branch --branch main` | 408.24 MiB | 409M | 10,685 | 42 |
| After: same command against the rewritten mirror | 845.24 KiB | 1.1M | 694 | 41 |

The working checkout's `.git` pack was 408.23 MiB before the rewrite. The full rewritten mirror, including rewritten tags and the three old pull-request branches, was 850.87 KiB.

Uncompressed unique blobs removed from history:

| Path | Blobs | Uncompressed size |
| --- | ---: | ---: |
| `src-tauri/target/` | 3,865 | 1848.23 MiB |
| `node_modules/` | 4,314 | 95.10 MiB |
| `graphify-out/` | 99 | 4.62 MiB |
| `src-tauri/gen/` | 3 | 0.21 MiB |
| `.claude/` | 1 | 862 bytes |

The pack is much smaller than the uncompressed total because Git delta-compresses those build outputs together. After the path removal, the largest remaining blobs are `src-tauri/icons/icon.icns` (about 190 KB) and historical copies of the lockfiles.

`docs/superpowers/` (about 56 KiB of design notes) is still in history. It is already listed in `.gitignore` and is not build output, so it was left in place.

## Paths removed

`git filter-repo --invert-paths` removed these directories from every rewritten commit:

- `node_modules/` — dependencies. Introduced in the scaffold commit and untracked again by `da20b02` (`chore: add .gitignore, remove node_modules and target from tracking`). The blobs stayed reachable from older commits.
- `src-tauri/target/` — Cargo and Tauri build output. Same history as `node_modules/`.
- `graphify-out/` — generated graph index. Already gitignored. Commit `808ecf56f86f567091163c58e210e04c2f14d970` (`chore: add graphify project index`, author Yong Qin) only added that directory, so the rewrite pruned it as an empty commit.
- `src-tauri/gen/` — generated Tauri schemas (`acl-manifests.json`, `capabilities.json`, `desktop-schema.json`). Already gitignored.
- `.claude/` — one local settings file, `.claude/settings.local.json`. Already gitignored. It contains local tool permissions and a developer machine path. It does not contain an API token or private key.

Commit messages, authors, author dates, committers, and committer dates were compared for every surviving `main` commit. None of them changed. No commit message was rewritten.

No credential material was found in the kept history or in text-like blobs under the removed paths (private keys, GitHub tokens, AWS access-key ids, Slack tokens, Google API keys). Unit tests use placeholder values such as `super-secret` and `access-secret`. Those tests were left as they are.

`.gitignore` already ignores every removed path, plus `dist/`, `*.db`, `docs/superpowers/`, and `.playwright-cli/`. No ignore rule was changed.

## Rewritten commits

Pre-cleanup `main`:

```text
834a9b57107c9382f98b7c5df2c9ed4163b36d86
tree 75072de5cf68f482e0509fcc3ebc5a3258238248
```

Rewritten `main` (same tree, generated paths removed from its ancestors):

```text
b28bad43a559ef13bc24edec6f7c417830a5890a
tree 75072de5cf68f482e0509fcc3ebc5a3258238248
```

`cursor/filter-repo-cleaned-f1f7` is that rewritten history plus the documentation commit that adds this file and `scripts/rewrite-git-history.sh`. Force-push that branch name, not the bare rewritten SHA, if this runbook should remain in the published history.

These tip blobs match the pre-cleanup tree:

| File | Blob |
| --- | --- |
| `src-tauri/src/modules/wecom_calendar.rs` | `0d4e40f64a` |
| `src-tauri/src/modules/feishu_calendar.rs` | `40e40dbf28` |
| `src-tauri/src/modules/google_calendar.rs` | `4409406eaa` |
| `src-tauri/src/modules/macos_calendar.rs` | `5a279a71b9` |
| `src/utils/wecomSync.ts` | `e2af7b5fba` |
| `.gitignore` | `9f4cf04f29` |
| `README.md` | `e8ab1028c3` |

Tags were rewritten locally. They were not pushed. Each new tag still points at an ancestor of the rewritten `main`:

| Tag | Current SHA | Rewritten SHA |
| --- | --- | --- |
| `v0.2.0` | `baefa9898902dd573cb49a1c6c58ecedff12acbc` | `ebd5f70593b84eb3731b792ca8228138c00e548d` |
| `v0.2.1` | `81da0e24da8b92742a1d74ed043aedffb8f994ce` | `4ec94221a5d27a17f9623d49a94cfffc8f20383c` |
| `v0.2.2` | `95051f85b0e58eb9e8a6176ec8885c9abc9c7938` | `6fdc36ce8c88ec48fd7dd89a15941c6fe039d333` |
| `v0.2.3` | `374d85e4f3fb123ccaf9962b4466446d07f6692a` | `2b8622039415ec55074c91a0be964ef7464523c0` |
| `v0.2.4` | `6a4e8edbf08b1b15f1105551448c34f773c564e8` | `bec8f19d58d94b94ce07349329a73575922c3627` |
| `v0.2.5` | `05cd86320d387ffa60a26d23a9635bd41bdcd79a` | `e5b2d90d01fdad60f3bacba0eba78150ca0cbc8a` |
| `v0.2.6` | `0c873747125175f41296ab7aaf0a161de4981db0` | `c8a073f1e49929c8852374f1c023064857525dba` |
| `v0.2.7` | `5375e0768ce5e110e0805936490a311bf338469d` | `c53decb44172412524994b05723c9b110d8ce459` |
| `v0.2.8` | `8c05bd8f89c67b19e087edd1192e6257dce1e9f4` | `be31feea89593e602ede67fda6cfe27844ea3fc6` |
| `v0.2.10` | `0bbfa2a6c36490d28ed7d69c4964cc0594a69fa1` | `065b9da28288f875071f1909296966bb1d4e186e` |
| `v0.2.11` | `fb357396e86e4197164fb89d57f684dd370b17bf` | `5b6c35fc7fb2b05d513280ace1b27535df8800dd` |
| `v0.2.12` | `84e3b664ba406ace1510b04cdc4585a763c8f01c` | `815ca7e5434dc188f1d070ad90167a7a693cdad9` |
| `v0.2.13` | `672ca105e954c0f350068050211b9ca994ab4b9d` | `7189aff87cf787092217ba4bc4d21b2f001ec6a4` |

## How to verify

On a fetch of `cursor/filter-repo-cleaned-f1f7`:

```bash
git fetch origin cursor/filter-repo-cleaned-f1f7
git rev-parse origin/cursor/filter-repo-cleaned-f1f7^
# parent is the rewritten main: b28bad43a559ef13bc24edec6f7c417830a5890a

git diff --stat 834a9b57107c9382f98b7c5df2c9ed4163b36d86 b28bad43a559ef13bc24edec6f7c417830a5890a
# empty: the trees match

git rev-list --objects b28bad43a559ef13bc24edec6f7c417830a5890a \
  | awk 'NF>1 { print $2 }' \
  | grep -E '^(node_modules/|src-tauri/target/|graphify-out/|src-tauri/gen/|\.claude/)' \
  || echo "removed paths absent"

git rev-parse 834a9b57107c9382f98b7c5df2c9ed4163b36d86:src-tauri/src/modules/wecom_calendar.rs \
              b28bad43a559ef13bc24edec6f7c417830a5890a:src-tauri/src/modules/wecom_calendar.rs
git rev-parse 834a9b57107c9382f98b7c5df2c9ed4163b36d86:src-tauri/src/modules/feishu_calendar.rs \
              b28bad43a559ef13bc24edec6f7c417830a5890a:src-tauri/src/modules/feishu_calendar.rs

git clone --no-local --bare --single-branch --branch cursor/filter-repo-cleaned-f1f7 \
  https://github.com/smileQiny/taskmanager.git /tmp/taskmanager-cleaned.git
git -C /tmp/taskmanager-cleaned.git count-objects -vH
du -sh /tmp/taskmanager-cleaned.git
```

To reproduce the rewrite instead of using the published branch:

```bash
python3 -m pip install --user git-filter-repo
export PATH="$HOME/.local/bin:$PATH"
scripts/rewrite-git-history.sh --dest /tmp/taskmanager-history-mirror.git
```

The script clones a mirror, runs the same `git filter-repo` invocation, and checks that the `main` tree is unchanged and that the removed paths are gone. It deletes the mirror's `origin` remote as part of filter-repo's safety behavior. It has no push step.

## Owner cutover

**Warning: the steps in this section rewrite published history. Every collaborator must re-clone. Open pull requests and local branches based on the old commit ids will not apply on top of the new history. Do not run them until the cleaned branch has been reviewed. The author of this branch did not force-push `main` and did not delete any remote branch.**

Publishing `cursor/filter-repo-cleaned-f1f7` by itself does not shrink GitHub. The old objects stay reachable from `main`, the release tags, and these branches, all of which still contain `src-tauri/target/`:

- `cursor/feishu-calendar-sync-3b08` (`808c2fed0c86cc07f9955a125191460d2a77e595`)
- `cursor/macos-calendar-sync-56e4` (`5b5d6a640981fd678295ff29a2b7e74b8c7524bc`)
- `cursor/quality-google-calendar-sync-5b00` (`6ec66f218282ec9367a7e12f8487682123177ac2`)

Those three branches are the pre-merge pull-request commits for Feishu (#3), macOS Calendar (#2), and Google Calendar (#1). The merged results are already on `main`, including WeCom (#4). Deleting the remote branches drops the original pull-request commits. It does not remove the merged sync code.

GitHub garbage-collects unreachable objects on its own schedule. A fresh clone shows the smaller size only after the fat refs are gone and that collection has run.

### 1. Freeze pushes and warn collaborators

Tell everyone to stop pushing to `main` and to the three branches above. After the cutover they should delete their old clones and clone the repository again. Rebasing old local commits onto the rewritten history is possible only by hand, using the trees, because the commit ids have changed.

### 2. Confirm `main` has not moved

The force-push below is pinned to the `main` that was rewritten. If this command fails, stop. New commits have to be replayed onto the cleaned history before publishing.

```bash
git fetch origin main
git rev-parse origin/main
# expected: 834a9b57107c9382f98b7c5df2c9ed4163b36d86
```

### 3. Force-push the cleaned branch onto `main`

```bash
git fetch origin cursor/filter-repo-cleaned-f1f7
git push origin cursor/filter-repo-cleaned-f1f7:main \
  --force-with-lease=main:834a9b57107c9382f98b7c5df2c9ed4163b36d86
```

`--force-with-lease` refuses the update when `origin/main` is no longer `834a9b57107c9382f98b7c5df2c9ed4163b36d86`.

### 4. Move the release tags

The rewritten tag commits are ancestors of the cleaned branch, so they are already in the repository after step 3. Moving a tag changes the commit GitHub uses for that release's source archive. Release assets uploaded as binaries stay attached to the release.

```bash
git push origin ebd5f70593b84eb3731b792ca8228138c00e548d:refs/tags/v0.2.0 --force
git push origin 4ec94221a5d27a17f9623d49a94cfffc8f20383c:refs/tags/v0.2.1 --force
git push origin 6fdc36ce8c88ec48fd7dd89a15941c6fe039d333:refs/tags/v0.2.2 --force
git push origin 2b8622039415ec55074c91a0be964ef7464523c0:refs/tags/v0.2.3 --force
git push origin bec8f19d58d94b94ce07349329a73575922c3627:refs/tags/v0.2.4 --force
git push origin e5b2d90d01fdad60f3bacba0eba78150ca0cbc8a:refs/tags/v0.2.5 --force
git push origin c8a073f1e49929c8852374f1c023064857525dba:refs/tags/v0.2.6 --force
git push origin c53decb44172412524994b05723c9b110d8ce459:refs/tags/v0.2.7 --force
git push origin be31feea89593e602ede67fda6cfe27844ea3fc6:refs/tags/v0.2.8 --force
git push origin 065b9da28288f875071f1909296966bb1d4e186e:refs/tags/v0.2.10 --force
git push origin 5b6c35fc7fb2b05d513280ace1b27535df8800dd:refs/tags/v0.2.11 --force
git push origin 815ca7e5434dc188f1d070ad90167a7a693cdad9:refs/tags/v0.2.12 --force
git push origin 7189aff87cf787092217ba4bc4d21b2f001ec6a4:refs/tags/v0.2.13 --force
```

### 5. Delete the stale branches that still hold the build output

Run this only after confirming nobody needs the original pull-request commits on those branches. This is a remote branch deletion. It is required for the clone-size reduction, and it is the owner's step.

```bash
git push origin --delete cursor/feishu-calendar-sync-3b08
git push origin --delete cursor/macos-calendar-sync-56e4
git push origin --delete cursor/quality-google-calendar-sync-5b00
```

### 6. Verify a fresh clone

```bash
rm -rf /tmp/taskmanager-after.git
git clone --bare https://github.com/smileQiny/taskmanager.git /tmp/taskmanager-after.git
git -C /tmp/taskmanager-after.git count-objects -vH
du -sh /tmp/taskmanager-after.git
git -C /tmp/taskmanager-after.git rev-list --objects --all \
  | awk 'NF>1 { print $2 }' \
  | grep -E '^(node_modules/|src-tauri/target/|graphify-out/)' \
  && echo "FAT PATHS STILL REACHABLE" || echo "fat paths absent"
```

Until GitHub collects the unreachable objects, the bare clone can stay large. If it is still about 408 MiB after the refs above are gone, ask GitHub Support to run garbage collection on `smileQiny/taskmanager`.

### 7. Collaborators re-clone

```bash
rm -rf taskmanager
git clone https://github.com/smileQiny/taskmanager.git
```

Local work that was committed on the old history has to be recreated as a patch against the new `main`. `git pull` will not fast-forward.
