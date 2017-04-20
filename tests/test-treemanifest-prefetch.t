  $ CACHEDIR=`pwd`/hgcache
  $ PYTHONPATH=$TESTDIR/..:$PYTHONPATH
  $ export PYTHONPATH

  $ . "$TESTDIR/library.sh"

  $ hg init master
  $ cd master
  $ mkdir dir
  $ echo x > dir/x
  $ hg commit -qAm 'add x'
  $ mkdir subdir
  $ echo z > subdir/z
  $ hg commit -qAm 'add subdir/z'
  $ echo x >> dir/x
  $ hg commit -Am 'modify x'
  $ cat >> .hg/hgrc <<EOF
  > [extensions]
  > treemanifest=
  > 
  > [remotefilelog]
  > name=master
  > cachepath=$CACHEDIR
  > usefastdatapack=True
  > 
  > [fastmanifest]
  > usetree=True
  > usecache=False
  > 
  > [treemanifest]
  > server=True
  > EOF

  $ cd ..
  $ hg clone ssh://user@dummy/master client
  streaming all changes
  4 files to transfer, 952 bytes of data
  transferred 952 bytes in * seconds (*) (glob)
  searching for changes
  no changes found
  updating to branch default
  2 files updated, 0 files merged, 0 files removed, 0 files unresolved

  $ cd master
  $ hg backfilltree

  $ cd ../client
  $ cat >> .hg/hgrc <<EOF
  > [extensions]
  > treemanifest =
  > fastmanifest = 
  > [remotefilelog]
  > reponame = master
  > cachepath = $CACHEDIR
  > [fastmanifest]
  > usetree = True
  > usecache = False
  > EOF

Test prefetchtrees
  $ hg prefetchtrees -r '0 + 1 + 2'
  $ ls $CACHEDIR/master/packs/manifests
  c0a571385df83107268172e0ab75abb30a12079a.histidx
  c0a571385df83107268172e0ab75abb30a12079a.histpack
  f074bc974cb09b3022f8bdd162950134af952bae.dataidx
  f074bc974cb09b3022f8bdd162950134af952bae.datapack
  $ hg debugdatapack --long $CACHEDIR/master/packs/manifests/*.dataidx
  
  dir
  Node                                      Delta Base                                Delta Length
  bc0c2c938b929f98b1c31a8c5994396ebb096bf0  0000000000000000000000000000000000000000  43
  
  subdir
  Node                                      Delta Base                                Delta Length
  ddb35f099a648a43a997aef53123bce309c794fd  0000000000000000000000000000000000000000  43
  
  
  Node                                      Delta Base                                Delta Length
  1be4ab2126dd2252dcae6be2aac2561dd3ddcda0  0000000000000000000000000000000000000000  95
  
  dir
  Node                                      Delta Base                                Delta Length
  a18d21674e76d6aab2edb46810b20fbdbd10fb4b  0000000000000000000000000000000000000000  43
  
  
  Node                                      Delta Base                                Delta Length
  60a7f7acb6bb5aaf93ca7d9062931b0f6a0d6db5  0000000000000000000000000000000000000000  95
  ef362f8bbe8aa457b0cfc49f200cbeb7747984ed  0000000000000000000000000000000000000000  46
  $ hg debughistorypack $CACHEDIR/master/packs/manifests/*.histidx
  
  
  Node          P1 Node       P2 Node       Link Node     Copy From
  60a7f7acb6bb  1be4ab2126dd  000000000000  bd6f9b289c01  
  1be4ab2126dd  ef362f8bbe8a  000000000000  f15c65c6e9bd  
  ef362f8bbe8a  000000000000  000000000000  ecfb693caff5  
  
  dir
  Node          P1 Node       P2 Node       Link Node     Copy From
  a18d21674e76  bc0c2c938b92  000000000000  bd6f9b289c01  
  bc0c2c938b92  000000000000  000000000000  ecfb693caff5  
  
  subdir
  Node          P1 Node       P2 Node       Link Node     Copy From
  ddb35f099a64  000000000000  000000000000  f15c65c6e9bd  
  $ hg debugdatapack --node ef362f8bbe8aa457b0cfc49f200cbeb7747984ed $CACHEDIR/master/packs/manifests/*.dataidx
  
  
  Node                                      Delta Base                                Delta SHA1                                Delta Length
  ef362f8bbe8aa457b0cfc49f200cbeb7747984ed  0000000000000000000000000000000000000000  3b295111780498d177793f9228bf736b915f0255  46
  $ hg -R ../master debugindex ../master/.hg/store/00manifesttree.i
     rev    offset  length  delta linkrev nodeid       p1           p2
       0         0      47     -1       0 ef362f8bbe8a 000000000000 000000000000
       1        47      61      0       1 1be4ab2126dd ef362f8bbe8a 000000000000
       2       108      58      1       2 60a7f7acb6bb 1be4ab2126dd 000000000000
  $ hg -R ../master debugindex ../master/.hg/store/00manifest.i
     rev    offset  length  delta linkrev nodeid       p1           p2
       0         0      48     -1       0 ef362f8bbe8a 000000000000 000000000000
       1        48      62      0       1 1be4ab2126dd ef362f8bbe8a 000000000000
       2       110      59      1       2 60a7f7acb6bb 1be4ab2126dd 000000000000

Test auto prefetch during normal access
  $ rm -rf $CACHEDIR/master
  $ ls $CACHEDIR/master/packs/manifests
  ls: cannot access $TESTTMP/hgcache/master/packs/manifests: No such file or directory
  [2]
  $ hg log -r tip --stat --pager=off
  changeset:   2:bd6f9b289c01
  tag:         tip
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     modify x
  
   dir/x |  1 +
   1 files changed, 1 insertions(+), 0 deletions(-)
  
  $ ls $CACHEDIR/master/packs/manifests
  148e9eb32f473ea522c591c95be0f9e772be9675.dataidx
  148e9eb32f473ea522c591c95be0f9e772be9675.datapack
  4ee15de76c068ec1c80e3e61f2c3c476a779078a.dataidx
  4ee15de76c068ec1c80e3e61f2c3c476a779078a.datapack
  6fbf331dc6290577b48479f94c2746754f4a898a.histidx
  6fbf331dc6290577b48479f94c2746754f4a898a.histpack
  a2b37afce1a72987e098a62a03ef89f3f208bc70.histidx
  a2b37afce1a72987e098a62a03ef89f3f208bc70.histpack
