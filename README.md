# CloneX

Repositories that contain other repositories' real histories.

A CloneX composition is an ordinary Git repository whose tree holds other
repositories (tools, libraries, fixtures) as plain directories, and whose
history holds their real commits — same SHAs, signatures, statuses. There
are no submodule pointers: a plain `git clone` has everything.

```bash
clonex declare tools/trunc --remote git@github.com:maxeonyx/trunc.git --follow main
# edit tools/trunc/… and tools/dotsync/… and the umbrella together; commit once
clonex publish          # one ordinary commit per affected repository
clonex sync             # adopt what others pushed (one merge)
clonex status           # per repository: what's here but not there, and vice versa
clonex where <change>   # where a logical change appears
```

Use jj (colocated) or plain Git for everything else. Design: `design/model.md`.
Status: experimental, not yet released.
