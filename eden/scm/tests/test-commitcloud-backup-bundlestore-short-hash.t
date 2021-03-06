#require py2
#chg-compatible

Create an ondisk bundlestore
  $ disable treemanifest
  $ . "$TESTDIR/library.sh"
  $ . "$TESTDIR/infinitepush/library.sh"
  $ cp $HGRCPATH $TESTTMP/defaulthgrc
  $ setupcommon
  $ hg init repo
  $ cd repo
  $ setupserver
  $ cd ..

Test `hg up` command for the commit that doesn't exist locally but does remotely.
We are making commit in repo (server) and will recover it in client 1 via short hash.

  $ hg clone ssh://user@dummy/repo client2 -q
  $ (cd repo && mkcommit somecommit && hg log -r .)
  commit:      f8b49bf62d4d
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     somecommit
  
  $ (cd ./client2 &&  hg up f8b49b)
  pulling 'f8b49b' from 'ssh://user@dummy/repo'
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

Test `hg up` command for the commit that doesn't exist locally
Also doesn't exist remotely
But can be recovered from backup
We are making a test commit in client 1 and will recover it from client2
We will also run few checks with `hg hide` / `hg up` commands.

  $ hg clone ssh://user@dummy/repo client1 -q
  $ cd client1
  $ mkcommit someothercommit
  $ hg log -r .
  commit:      c1b6fe8fce73
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     someothercommit
  
Backup commit
  $ hg cloud backup
  backing up stack rooted at f8b49bf62d4d
  commitcloud: backed up 1 commit
  remote: pushing 2 commits:
  remote:     f8b49bf62d4d  somecommit
  remote:     c1b6fe8fce73  someothercommit

Quick test `hg hide` / `hg up`
Check update now accesses hidden commits rather than trying to pull
  $ cat >> .hg/hgrc << EOF
  > [extensions]
  > amend=
  > directaccess=
  > [experimental]
  > evolution=exchange
  > evolution.createmarkers=True
  > EOF
  $ hg hide c1b6fe8fce73
  hiding commit c1b6fe8fce73 "someothercommit"
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved
  working directory now at f8b49bf62d4d
  1 changeset hidden
  $ hg up c1b6fe8fce73
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

Test that updating to new head after hiding current head works as expected.
  $ hg up -q ".^"
  $ mkcommit newheadcommit
  $ hg hide -r "."
  hiding commit 5862354b0f4f "newheadcommit"
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved
  working directory now at f8b49bf62d4d
  1 changeset hidden
  $ hg up -qr "heads(.::)"
  $ hg log -r "." -T "{node|short}\n"
  f8b49bf62d4d

Check hg up on another client.
Commit should be pulled from backup storage.
  $ (cd ../client2 && hg up c1b6fe)
  pulling 'c1b6fe' from 'ssh://user@dummy/repo'
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

  $ cd ..

Test pulling a commit with the same prefix by creating fake files
  $ echo ' ' > ./repo/.hg/scratchbranches/index/nodemap/b1b6fe8fce73221de4162469dac9a6f8d01744a1
  $ echo ' ' > ./repo/.hg/scratchbranches/index/nodemap/b1b6fe8fce73221de4162469dac9a6f8d01744a2
  $ (cd ./client2 && hg up b1b6fe)
  pulling 'b1b6fe' from 'ssh://user@dummy/repo'
  pull failed: ambiguous identifier 'b1b6fe'
  suggestion: provide longer commithash prefix
  abort: unknown revision 'b1b6fe'!
  [255]

Clean up from the previous tests
  $ rm -r ./client1
  $ rm -r ./client2

Set up similar test but with sql infinitepush storage
The test scenario will cover several different lengths of prefix

#if no-osx
  $ now=`date +%s`
  $ then30=`expr $now - 30 \* 24 \* 60 \* 60`
  $ then32=`expr $now - 32 \* 24 \* 60 \* 60`
  $ mkcommitveryold() {
  >    echo "$1" > "$1"
  >    hg add "$1"
  >    hg ci -d "0 0" -m "$1"
  > }
  $ mkcommitrecent() {
  > echo "$1" > "$1"
  > hg add "$1"
  > hg ci -m "$1" -d"$now 0"
  > }
  $ mkcommitsold32days() {
  > echo "$1" > "$1"
  > hg add "$1"
  > hg ci -m "$1" -d "$then32 0"
  > }
  $ mkcommitsold30days() {
  > echo "$1" > "$1"
  > hg add "$1"
  > hg ci -m "$1" -d "$then30 0"
  > }
  $ . "$TESTDIR/infinitepush/library.sh"
  $ setupcommon

With no configuration it should abort
  $ hg init server
  $ cd server
  $ setupsqlserverhgrc babar
  $ echo "[infinitepush]" >> .hg/hgrc
  $ echo "shorthasholdrevthreshold=31" >> .hg/hgrc
  $ setupdb
  $ cd ..
  $ hg clone -q ssh://user@dummy/server client1
  $ hg clone -q ssh://user@dummy/server client2
  $ cd ./client1
  $ setupsqlclienthgrc
  $ cd ../client2
  $ setupsqlclienthgrc
  $ cd ../client1

  $ mkcommitrecent someothercommit1
  $ my_new_commit1=`hg parent --template '{node}'`
  $ my_new_commit1_hashlen6=`echo $my_new_commit1 | fold -w 6 | head -n 1`

  $ mkcommitrecent someothercommit2
  $ my_new_commit2=`hg parent --template '{node}'`
  $ my_new_commit2_hashlen5=`echo $my_new_commit2 | fold -w 5 | head -n 1`

  $ mkcommitveryold someothercommit3
  $ my_new_commit3=`hg parent --template '{node}'`
  $ my_new_commit3_hashlen9=`echo $my_new_commit3 | fold -w 9 | head -n 1`

  $ mkcommitsold32days someothercommit4
  $ my_new_commit4=`hg parent --template '{node}'`
  $ my_new_commit4_hashlen12=`echo $my_new_commit4 | fold -w 12 | head -n 1`

  $ mkcommitsold30days someothercommit5
  $ my_new_commit5=`hg parent --template '{node}'`
  $ my_new_commit5_hashlen10=`echo $my_new_commit5 | fold -w 10 | head -n 1`

  $ hg cloud backup
  backing up stack rooted at * (glob)
  commitcloud: backed up 5 commits
  remote: pushing 5 commits:
  remote:     *  someothercommit1 (glob)
  remote:     *  someothercommit2 (glob)
  remote:     *  someothercommit3 (glob)
  remote:     *  someothercommit4 (glob)
  remote:     *  someothercommit5 (glob)
  $ cd ../

case 1: recent commit, length of prefix = 6 characters
  $ (cd ./client2 && hg up $my_new_commit1_hashlen6)
  pulling * from 'ssh://user@dummy/server' (glob)
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

case 2: recent commit, length of prefix < 6 characters
  $ (cd ./client2 && hg up $my_new_commit2_hashlen5)
  pulling '*' from 'ssh://user@dummy/server' (glob)
  abort: unknown revision '*'! (glob)
  [255]

case 3: test longerlength
in this case we also test pulling old commits

case 3a: very old commit, hash size 9 characters
  $ (cd ./client2 && hg up $my_new_commit3_hashlen9)
  pulling * from 'ssh://user@dummy/server' (glob)
  pull failed: commit '*' is more than 31 days old (glob)
  description:
    changeset: * (glob)
    author: test
    date: 01 Jan 1970 00:00
    summary: someothercommit3
  #commitcloud hint: if you would like to fetch this commit, please provide the full hash
  abort: unknown revision '*'! (glob)
  [255]

case 3b: 32 days old commit, hash size 12 characters
  $ (cd ./client2 && hg up $my_new_commit4_hashlen12)
  pulling * from 'ssh://user@dummy/server' (glob)
  pull failed: commit '*' is more than 31 days old (glob)
  description:
    changeset: * (glob)
    author: test
    date: * (glob)
    summary: someothercommit4
  #commitcloud hint: if you would like to fetch this commit, please provide the full hash
  abort: unknown revision '*'! (glob)
  [255]

case 3ba: same test but check that output contains the full hash
  $ (cd ./client2 && hg up $my_new_commit4_hashlen12 2>&1 | grep $my_new_commit4)
  * changeset: * (glob)

case 3b: 32 days old commit, hash size - full hash
  $ (cd ./client2 && hg up $my_new_commit4)
  pulling * from 'ssh://user@dummy/server' (glob)
  3 files updated, 0 files merged, 0 files removed, 0 files unresolved

case 3c: 30 days old, hash size 10 characters
  $ (cd ./client2 && hg up $my_new_commit5_hashlen10)
  pulling * from 'ssh://user@dummy/server' (glob)
  1 files updated, 0 files merged, 0 files removed, 0 files unresolved

case 3ba: 32 days old commit, hash size 12 characters but it was already uploaded
so, it is just local switch
  $ (cd ./client2 && hg up $my_new_commit4_hashlen12)
  0 files updated, 0 files merged, 1 files removed, 0 files unresolved

case 3d: commit doesn't exists in the DB
Test when the commit is not found
  $ (cd ./client2 && hg up aaaaaa)
  pulling 'aaaaaa' from 'ssh://user@dummy/server'
  pull failed: unknown revision 'aaaaaa'
  abort: unknown revision 'aaaaaa'!
  [255]

#endif
