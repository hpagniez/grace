---
status: accepted
date: 2026-11-01
decision-makers: Tusquito
---

# Sign Grace.exe with SignPath Foundation

## Context and Problem Statement

The release zip is unsigned ([ADR 0007](0007-github-actions-ci-cd-on-the-free-private-plan.md), [ADR 0018](0018-make-the-repository-public.md)). Windows marks a zip downloaded with a browser, so the first start of `Grace.exe` meets SmartScreen's "Windows protected your PC" until the person clicks **More info**, then **Run anyway**, and Windows names no publisher. [ADR 0019](0019-update-the-zip-install-from-github-releases.md) keeps the GitHub account as the updater's trust root and asks for a record before a signed build.

In-app updates are not affected: the updater downloads the zip itself, without the mark of the web, so SmartScreen never sees it.

How does Grace get a code signing certificate, and where is the signing done? Tracked in [#176](https://github.com/hpagniez/grace/issues/176).

## Considered Options

1. SignPath Foundation: free signing for open-source projects, from the GitHub Actions build.
2. Azure Artifact Signing (Trusted Signing), about 10 dollars a month.
3. A commercial OV certificate with its key in the CA's cloud HSM (for example SSL.com eSigner or DigiCert KeyLocker).
4. The Microsoft Store, which signs MSIX packages itself.
5. Stay unsigned.

## Decision Outcome

Chosen: option 1.

- **What is signed.** `Grace.exe` only: it is the program the person starts, and the only one built from this repository. The Java runtime's programs already carry the Eclipse Foundation's signature; Electron's DLLs ship as Electron publishes them. The Foundation's terms allow bundled upstream binaries unsigned, and forbid signing them as ours.
- **Where.** The `release` workflow has three jobs. `build` makes the zip with a read-only token and uploads it as a workflow artifact. `sign` sends it to SignPath with `SignPath/github-action-submit-signing-request`, which proves to SignPath that this repository's workflow built it, and gets it back with `Grace.exe` signed. `release`, the only job with `contents: write`, publishes the zip.
- **Third-party action.** SignPath's action is the one action not owned by GitHub. It is pinned by full commit SHA and allowed by that SHA in the repository's Actions settings. It runs in its own job with `actions: read` and `contents: read`, so it never holds the token that can publish.
- **Approval.** SignPath Foundation requires a person to approve each release signing in SignPath. The `sign` job waits up to an hour, then fails, and nothing is published.
- **Metadata.** The artifact configuration, [`.signpath/artifact-configuration.xml`](../../.signpath/artifact-configuration.xml), signs only `Grace.exe` inside the zip, and requires its product name to be `Grace` and its product version to equal `package.json`'s.
- **Until accepted.** The `sign` job runs only once the repository has the `SIGNPATH_ORGANIZATION_ID` variable. Until then, releases are built and published unsigned, as before.
- **By hand.** Running the workflow by hand builds and signs with SignPath's test certificate, and publishes nothing: it checks the setup without a release.
- **Publisher.** Signed files show SignPath Foundation as publisher, with Grace as the product. The README's code signing policy names the team roles and holds the privacy policy the Foundation requires; every release links to it.
- **Updater.** Unchanged: the zip's name, layout and SHA-256 check stay as ADR 0019 set them. Refusing an unsigned `Grace.exe` before switching would be stronger, but it would refuse every unsigned release, so it needs its own record.

Why not the others:

- Option 2: individual developers must be in the United States or Canada, and onboarding of new individuals has been reported paused. The Student Developer Pack's Azure credit comes as a sponsored subscription, which Artifact Signing reportedly does not take.
- Option 3: works in CI, but costs a few hundred dollars a year for a free app.
- Option 4: replaces the zip and the in-app updater (ADR 0019) with a Store package.
- Option 5: keeps the warning on every first start.

### Consequences

- Good: Windows names a publisher, and every signature ties to a build of this repository.
- Good: free, and the build stays in GitHub Actions.
- Neutral: SmartScreen trusts by reputation, earned per certificate and per file over time, so a signed release may still warn at first.
- Bad: one third-party action and one external service in the release path. If SignPath is down, a release waits; if the Foundation stops the project, removing the variable goes back to unsigned releases.
- Bad: every release waits for a person to approve it in SignPath.

## More Information

- [SignPath Foundation terms](https://signpath.org/terms)
- [SignPath: GitHub trusted build system](https://docs.signpath.io/trusted-build-systems/github)
- [SignPath: artifact configuration syntax](https://docs.signpath.io/artifact-configuration/syntax)
- [Microsoft: code signing options for Windows app developers](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
- [Microsoft: set up Artifact Signing](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart)
- [Getting started with Azure Artifact Signing](https://www.itwriting.com/blog/12462-getting-started-with-azure-artifact-signing.html) (sponsored subscriptions)
