#chg-compatible

Setup

  $ hg init repo
  $ cd repo
  $ echo base > base
  $ hg add base
  $ hg commit -m "base"

Deliberately corrupt the dirstate.

  $ dd if=/dev/zero bs=4096 count=1 of=.hg/dirstate 2> /dev/null
  $ hg debugrebuilddirstate
  warning: failed to inspect working copy parent
