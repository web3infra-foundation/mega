# Audit

Run `buck2 audit` with all the special arguments encouraged for use with BTD.
Using this helper ensures that as BTD evolves your scripts will continue to work
correctly.

As an example:

```shell
supertd audit cells
supertd audit config
```

Within Meta a precompiled version of `supertd` is available as devfeature at
`/usr/local/bin/supertd`.

Devfeature should install automatically on devservers and ondemands, but could also be installed explicitly via `feature install supertd`.
