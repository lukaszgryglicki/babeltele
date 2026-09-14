# Release Notes — version 2.14

## Summary

This release focuses on **performance improvements** and a couple of important bug fixes. Please read the [migration guide](https://example.com/migrate) before upgrading.

## Changes

- The payment endpoint is now approximately 40 percent faster because we removed a redundant database query.
- Fixed a bug where the status page did not refresh automatically.
- [ ] Documentation for the new API is still in progress.

| Component | Status | Notes |
|-----------|--------|-------|
| API | done | deployed to production on Monday |
| Web UI | in progress | waiting for the design review |

> Note: the old endpoint will be removed in the next major version.

```python
def total(items):
    return sum(i.price for i in items)  # sum of all prices
```
