# git hooks

## Justification

If you find it annoying to wait for CI on GitHub to tell you that you forgot to format your code or generate the templated files, you might find it convenient to have this error happen earlier.

## Enabling the hooks

```bash
git config --local core.hooksPath .githooks
```
