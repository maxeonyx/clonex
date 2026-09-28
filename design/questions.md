# Questions for the owner

These are input to the design, not decisions. Answers so far (2026-09-28),
treated as fallible input:
- Trailers in commits are fine.
- Committing ref/branch names into history breaks the model.
- The umbrella stays a test project until CloneX is seriously proven; it
  must beat Git and jj on *all* aspects.
- Publish the repo (done); `cx` is just an alias.
- The transaction language is deferred.

Open (each links the cases that make it matter):

1. **"No names in history", or "nothing ever resolves a name from
   history"?** Git's merge messages contain branch names harmlessly. The
   design meets the stricter form. The weaker one would allow informative
   text such as "Adopt trunc main". Cases: `research/refs-attack.md` O1.
2. **Private ref namespaces on GitHub.** `refs/meta/clonex` (config),
   `refs/clonex/topics/*` (bindings) and `refs/clonex/adopted/*` (anchors)
   are invisible to plain Git and unprotected: anyone with push access can
   write them. The continuity check limits the damage. Is that acceptable,
   or should this state live in a service? Cases: refs-attack F6, O3.
3. **Any state or service outside GitHub?** Only something running at merge
   time can retarget stacked PRs before delete-on-merge closes them, and
   show in-flight work across clones. Cases: refs-attack F7, W4;
   refs-workflows T11.
4. **Branch naming.** Should a name be required when work starts (as with
   `at-<slug>` today), or may unnamed jj work publish under generated
   names? Is one name per logical change a default or a rule? Cases:
   refs-workflows T2/T3.
5. **Merge-commit-only as an ecosystem assumption?** That makes "did this
   land?" an ancestry question, with squash/rebase-merge detection as best
   effort. Cases: refs-workflows T13; audit REG-2.
6. **May a composition trunk ever publish directly to component trunks?**
   The default is now no: component trunks move only through their own
   PRs. Cases: refs-attack F3, O6.
