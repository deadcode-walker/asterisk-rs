# Repository knowledge

This directory is the versioned system of record for asterisk-rs. Public user documentation lives
in `src/`; engineering knowledge lives in the focused files and directories below.

| Location | Status | Owner | Refresh trigger |
|---|---|---|---|
| `src/` | public, active | protocol maintainers | public API or behavior changes |
| `design-docs/` | approved decisions | maintainers | a recorded constraint or decision changes |
| `product-specs/` | behavior map | maintainers | a protocol surface or support promise changes |
| `exec-plans/` | living work records | plan owner | every material stopping point |
| `generated/` | reproducible references | generating command | generator input changes |
| `references/` | pinned external knowledge index | maintainers | upstream version or contract changes |
| `QUALITY_SCORE.md` | current evidence and gaps | maintainers | full review or gate changes |
| `RELIABILITY.md` | failure and recovery model | protocol maintainers | lifecycle/concurrency changes |
| `SECURITY.md` | engineering trust boundaries | security maintainer | boundary or dependency changes |

Delete stale documents instead of retaining competing truths.
