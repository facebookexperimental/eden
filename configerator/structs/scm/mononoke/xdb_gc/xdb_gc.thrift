// @generated SignedSource<<a732b72d175a75f104e36a1b5814e232>>
// DO NOT EDIT THIS FILE MANUALLY!
// This file is a mechanical copy of the version in the configerator repo. To
// modify it, edit the copy in the configerator repo instead and copy it over by
// running the following in your fbcode directory:
//
// configerator-thrift-updater scm/mononoke/xdb_gc/xdb_gc.thrift

/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

namespace py configerator.mononoke.xdb_gc

struct XdbGc {
    1: i64 put_generation,
    2: i64 mark_generation,
    3: i64 delete_generation,
}
