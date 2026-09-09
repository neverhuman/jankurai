# GitHub Marketplace Action

`action.yml` is the Marketplace entry. It installs the released auditor, runs
`jankurai audit`, and fails the job when the score is below `fail-under`
(default 85). Raise the floor to 90, or set `0` to keep artifacts only.

```yaml
name: jankurai
on:
  pull_request:
  push:
    branches: [main]
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: neverhuman/jankurai@v1.7.0
        with:
          fail-under: 85
```

Publish from a tagged release of this repository. The Action uses the same
verified installer as the README.
