# Project documentation

- [Architecture](../ARCHITECTURE.md): protocol paths, crate boundaries, and build tools.
- [Contributing](../CONTRIBUTING.md): development commands, tests, and pull requests.
- [User guide](src/SUMMARY.md): AMI, AGI, and ARI usage; rustdoc owns the API reference.
- [Reliability](RELIABILITY.md): timeouts, resource bounds, recovery, and live tests.
- [Security](SECURITY.md): code and automation trust boundaries.
- [Vulnerability reporting](../SECURITY.md): public disclosure policy.
- [References](references/index.md): pinned protocol artifacts and 0.8 compatibility notes.

Update affected guides, rustdoc, examples, and migration notes alongside behavior changes. Run
`just docs-check` to compile the representative snippets and build rustdoc and the mdBook.

release-plz owns package changelogs and the umbrella release body. Ordinary changes use conventional
commits and do not add speculative `Unreleased` entries. Review release PRs for package scope,
breaking changes, links, and duplicates.
