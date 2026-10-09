# Local package candidate and independent adoption

Decision: prepare and verify an immutable **local review artifact** before any registry or hosted release. No registry publication, tag, Git push, PR merge, signing service or production deployment is authorized by this work. Public release channel/semver is a separate confirmation; local package version `0.1.0` retains the existing engine report version and does not make changed archive bytes interchangeable with an earlier `0.1.0` build.

For local review, identify a candidate with all of: repository commit, package name/version, exact archive SHA-256, lockfile SHA-256 and build toolchain/platform. Version alone is insufficient. Store archives by content digest; never replace an archive under an existing immutable digest. The package's integration wire remains `guard.integration/v1alpha1`; the older engine wire remains `guard.partme.ai/v1alpha1`. Policy revision and content digest are independent of both.

Verification procedure:

1. From a clean commit, run `cargo package --locked --offline`. Cargo must verify the packaged source.
2. Record the source commit, package/lock hashes, Rust/Cargo versions and platform. Copy the archive to an independent artifact directory.
3. Extract it outside all seven sibling worktrees. Build a fresh consumer against that extracted package with an exact package version and a separately generated locked dependency graph. Test supported envelope profiles and rejection of a stronger unsupported profile. Inspect the consumer lock to verify that no sibling source path is present.
4. Build/run the CLI from the extracted package. Preserve observed command, exit and fixture digests. Tests of this Linux artifact do not qualify other platforms.

Rollback: select the earlier immutable package plus its recorded lock/toolchain; restore the adapter/controller independently. Preserve previously stored reports and their original engine package provenance. Archived reports must be recomputed with the matching supported engine version; do not silently rewrite their reported engine version, wire version, evidence digests or approvals to accommodate an upgrade. A failed/unknown compatibility matrix blocks enforcement rather than falling back to permissive behavior.

This local evidence can prove independent consumption without `../guardengine`. It does not prove a public crate/binary exists, that package-manager signatures authenticate a producer, or that all six consumers are release-qualified. Registry ownership, final public version, platform signing and hosted rollout remain explicit release decisions. Current local dependency adoption may still use sibling paths until the independently pinned consumer matrix is accepted.
